from __future__ import annotations

import base64
import importlib
import json
import os
import shutil
import subprocess
import urllib.parse
import urllib.request
from pathlib import Path


_EXTERNAL_SESSION_CACHE: dict[tuple[str, bool, str], "ExternalRuntimeSession"] = {}
_RUNTIME_MODE = "auto"
_RUNTIME_LOCAL_PYTHON = ""
_NEXT_GEN_MIN_VERSION = "2.0.0"


def set_runtime_preferences(mode: str = "auto", local_python: str = "") -> None:
    """Set runtime interpreter selection preferences for this plugin process.

    mode values:
    - auto: prefer discovered external interpreter, then current runtime
    - local: force a caller-provided local interpreter path
    - qgis: force current in-process Python runtime
    """
    global _RUNTIME_MODE, _RUNTIME_LOCAL_PYTHON
    normalized = str(mode or "auto").strip().lower()
    if normalized not in {"auto", "local", "qgis"}:
        normalized = "auto"
    _RUNTIME_MODE = normalized
    _RUNTIME_LOCAL_PYTHON = str(local_python or "").strip()


def get_runtime_preferences() -> tuple[str, str]:
    return _RUNTIME_MODE, _RUNTIME_LOCAL_PYTHON


class RuntimeBootstrapError(RuntimeError):
    pass


def _next_gen_required_message(detail: str = "") -> str:
    base = (
        "This plugin requires whitebox_workflows Next Gen (v2.x) with RuntimeSession capabilities support. "
        "A legacy whitebox_workflows runtime (v1.x) was detected. Install/activate a v2.x build and retry."
    )
    detail = str(detail or "").strip()
    if detail:
        return f"{base} Detail: {detail}"
    return base


def _is_pro_unavailable_error(message: str) -> bool:
    text = str(message or "").lower()
    return "include_pro=true requested" in text and "does not include pro support" in text


def _is_legacy_runtime_error(message: str) -> bool:
    text = str(message or "").lower()
    checks = (
        "legacy whitebox_workflows runtime",
        "requires whitebox_workflows next gen",
        "unexpected keyword argument 'include_pro'",
        "unexpected keyword argument 'tier'",
        "has no attribute 'runtimesession'",
        "object has no attribute 'get_runtime_capabilities_json'",
        "unsupported method: get_runtime_capabilities_json",
    )
    return any(token in text for token in checks)


def _parse_next_gen_capabilities(raw: str, runtime_label: str) -> dict:
    try:
        parsed = json.loads(raw)
    except Exception as exc:
        raise RuntimeBootstrapError(
            _next_gen_required_message(
                f"{runtime_label} returned non-JSON capabilities payload ({exc})."
            )
        ) from exc

    if not isinstance(parsed, dict):
        raise RuntimeBootstrapError(
            _next_gen_required_message(
                f"{runtime_label} returned invalid capabilities type: {type(parsed).__name__}."
            )
        )

    required_keys = ("runtime_mode", "effective_tier", "requested_tier")
    missing = [key for key in required_keys if key not in parsed]
    if missing:
        raise RuntimeBootstrapError(
            _next_gen_required_message(
                f"{runtime_label} capabilities missing keys: {', '.join(missing)}."
            )
        )

    return parsed


class ExternalRuntimeSession:
    def __init__(self, python_executable: str, include_pro: bool, tier: str):
        self._python_executable = python_executable
        self._include_pro = include_pro
        self._tier = tier
        self._stream_worker_process: subprocess.Popen | None = None

    def _build_clean_env(self) -> dict[str, str]:
        clean_env = dict(os.environ)
        clean_env.pop("PYTHONHOME", None)
        clean_env.pop("PYTHONPATH", None)
        return clean_env

    def _build_stream_worker_runner(self) -> str:
        return (
            "import base64, json, sys, traceback\n"
            "import whitebox_workflows as wbw\n"
            "include_pro = bool(json.loads(sys.argv[1]))\n"
            "tier = str(sys.argv[2])\n"
            "session = wbw.RuntimeSession(include_pro=include_pro, tier=tier)\n"
            "def _emit(prefix, text):\n"
            "    enc = base64.b64encode(text.encode('utf-8')).decode('ascii')\n"
            "    sys.stdout.write(prefix + enc + '\\n')\n"
            "    sys.stdout.flush()\n"
            "def _emit_event(evt):\n"
            "    try:\n"
            "        if isinstance(evt, str):\n"
            "            payload = evt\n"
            "        else:\n"
            "            payload = json.dumps(evt)\n"
            "    except Exception as exc:\n"
            "        payload = json.dumps({'type': 'message', 'message': str(exc)})\n"
            "    _emit('__WBW_WORKER_EVENT__', payload)\n"
            "for raw in sys.stdin:\n"
            "    line = raw.strip()\n"
            "    if not line:\n"
            "        continue\n"
            "    try:\n"
            "        cmd = json.loads(line)\n"
            "    except Exception as exc:\n"
            "        _emit('__WBW_WORKER_ERROR__', f'invalid command json: {exc}')\n"
            "        continue\n"
            "    action = str(cmd.get('action', ''))\n"
            "    if action == 'shutdown':\n"
            "        _emit('__WBW_WORKER_OK__', 'shutdown')\n"
            "        break\n"
            "    if action != 'run_tool_json_stream':\n"
            "        _emit('__WBW_WORKER_ERROR__', f'unsupported action: {action}')\n"
            "        continue\n"
            "    tool_id = str(cmd.get('tool_id', ''))\n"
            "    args_json = str(cmd.get('args_json', '{}'))\n"
            "    try:\n"
            "        try:\n"
            "            out = session.run_tool_json_stream(tool_id, args_json, _emit_event)\n"
            "        except TypeError:\n"
            "            out = session.run_tool_json_stream(tool_id, args_json)\n"
            "        except Exception:\n"
            "            out = session.run_tool_json(tool_id, args_json)\n"
            "        out_text = out if isinstance(out, str) else json.dumps(out)\n"
            "        _emit('__WBW_WORKER_RESULT__', out_text)\n"
            "    except Exception as exc:\n"
            "        _emit('__WBW_WORKER_ERROR__', traceback.format_exc())\n"
        )

    def _start_stream_worker(self) -> None:
        if self._stream_worker_process is not None and self._stream_worker_process.poll() is None:
            return

        runner = self._build_stream_worker_runner()
        self._stream_worker_process = subprocess.Popen(
            [self._python_executable, "-c", runner, json.dumps(self._include_pro), self._tier],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            env=self._build_clean_env(),
            bufsize=1,
        )

    def _stop_stream_worker(self) -> None:
        process = self._stream_worker_process
        self._stream_worker_process = None
        if process is None:
            return

        try:
            if process.poll() is None and process.stdin is not None:
                process.stdin.write(json.dumps({"action": "shutdown"}) + "\n")
                process.stdin.flush()
        except Exception:
            pass

        try:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=2.0)
        except Exception:
            try:
                process.kill()
            except Exception:
                pass

    def __del__(self):
        self._stop_stream_worker()

    def _invoke(self, method: str, **kwargs) -> str:
        payload = {
            "method": method,
            "include_pro": self._include_pro,
            "tier": self._tier,
        }
        payload.update(kwargs)
        runner = (
            "import json, sys\n"
            "import whitebox_workflows as wbw\n"
            "p = json.loads(sys.argv[1])\n"
            "s = wbw.RuntimeSession(include_pro=bool(p.get('include_pro', True)), tier=str(p.get('tier', 'open')))\n"
            "m = p.get('method')\n"
            "if m == 'get_runtime_capabilities_json':\n"
            "    out = s.get_runtime_capabilities_json()\n"
            "elif m == 'list_tool_catalog_json':\n"
            "    out = s.list_tool_catalog_json()\n"
            "elif m == 'get_tool_metadata_json':\n"
            "    out = s.get_tool_metadata_json(str(p.get('tool_id', '')))\n"
            "elif m == 'run_tool_json_stream':\n"
            "    tool_id = str(p.get('tool_id', ''))\n"
            "    args_json = str(p.get('args_json', '{}'))\n"
            "    try:\n"
            "        out = s.run_tool_json_stream(tool_id, args_json, lambda _evt: None)\n"
            "    except TypeError:\n"
            "        out = s.run_tool_json_stream(tool_id, args_json)\n"
            "elif m == 'run_projection_wrapper_json':\n"
            "    tool_id = str(p.get('tool_id', ''))\n"
            "    args = json.loads(str(p.get('args_json', '{}')))\n"
            "    wbe = wbw.WbEnvironment()\n"
            "    epsg = int(args.get('epsg'))\n"
            "    in_path = str(args.get('input', ''))\n"
            "    out_path = str(args.get('output', ''))\n"
            "    if tool_id == 'reproject_raster':\n"
            "        resample = str(args.get('resample', 'bilinear')).strip() or 'bilinear'\n"
            "        src = wbe.read_raster(in_path)\n"
            "        out_obj = wbe.reproject_raster(src, dst_epsg=epsg, resample=resample)\n"
            "        wbe.write_raster(out_obj, out_path)\n"
            "    elif tool_id == 'reproject_lidar':\n"
            "        src = wbe.read_lidar(in_path)\n"
            "        out_obj = wbe.reproject_lidar(src, dst_epsg=epsg)\n"
            "        wbe.write_lidar(out_obj, out_path)\n"
            "    elif tool_id == 'assign_projection_raster':\n"
            "        src = wbe.read_raster(in_path)\n"
            "        src.set_crs_epsg(epsg)\n"
            "        if not out_path:\n"
            "            out_path = in_path\n"
            "        wbe.write_raster(src, out_path)\n"
            "    elif tool_id == 'assign_projection_vector':\n"
            "        src = wbe.read_vector(in_path)\n"
            "        src.set_crs_epsg(epsg)\n"
            "        if not out_path:\n"
            "            out_path = in_path\n"
            "        wbe.write_vector(src, out_path)\n"
            "    elif tool_id == 'assign_projection_lidar':\n"
            "        src = wbe.read_lidar(in_path)\n"
            "        src.set_crs_epsg(epsg)\n"
            "        if not out_path:\n"
            "            out_path = in_path\n"
            "        wbe.write_lidar(src, out_path)\n"
            "    else:\n"
            "        raise RuntimeError(f'Unsupported projection wrapper tool: {tool_id}')\n"
            "    out = {'outputs': {'output': out_path or in_path}}\n"
            "else:\n"
            "    raise RuntimeError(f'Unsupported method: {m}')\n"
            "if isinstance(out, str):\n"
            "    sys.stdout.write(out)\n"
            "else:\n"
            "    sys.stdout.write(json.dumps(out))\n"
        )

        completed = subprocess.run(
            [self._python_executable, "-c", runner, json.dumps(payload)],
            check=False,
            capture_output=True,
            text=True,
            env=self._build_clean_env(),
        )
        if completed.returncode != 0:
            stderr = completed.stderr.strip() or completed.stdout.strip() or "unknown external runtime error"
            raise RuntimeBootstrapError(
                "External whitebox_workflows runtime failed via "
                f"{self._python_executable}: {stderr}"
            )
        return completed.stdout

    def get_runtime_capabilities_json(self):
        return self._invoke("get_runtime_capabilities_json")

    def list_tool_catalog_json(self):
        return self._invoke("list_tool_catalog_json")

    def get_tool_metadata_json(self, tool_id: str):
        return self._invoke("get_tool_metadata_json", tool_id=tool_id)

    def run_tool_json_stream(self, tool_id: str, args_json: str, _callback=None):
        try:
            return self._run_tool_json_stream_persistent(tool_id, args_json, _callback)
        except Exception:
            self._stop_stream_worker()
            return self._run_tool_json_stream_oneoff(tool_id, args_json, _callback)

    def _run_tool_json_stream_persistent(self, tool_id: str, args_json: str, _callback=None) -> str:
        self._start_stream_worker()
        process = self._stream_worker_process
        if process is None or process.stdin is None or process.stdout is None:
            raise RuntimeBootstrapError("External runtime worker failed to start.")

        command = {
            "action": "run_tool_json_stream",
            "tool_id": tool_id,
            "args_json": args_json,
        }
        process.stdin.write(json.dumps(command) + "\n")
        process.stdin.flush()

        loose_stdout_lines: list[str] = []

        while True:
            raw_line = process.stdout.readline()
            if raw_line == "":
                break
            line = raw_line.rstrip("\r\n")
            if line.startswith("__WBW_WORKER_EVENT__"):
                enc = line[len("__WBW_WORKER_EVENT__"):]
                try:
                    evt = base64.b64decode(enc).decode("utf-8", errors="replace")
                except Exception:
                    continue
                if callable(_callback):
                    try:
                        _callback(evt)
                    except Exception:
                        pass
                continue
            if line.startswith("__WBW_WORKER_RESULT__"):
                enc = line[len("__WBW_WORKER_RESULT__"):]
                try:
                    return base64.b64decode(enc).decode("utf-8", errors="replace")
                except Exception:
                    return "{}"
            if line.startswith("__WBW_WORKER_ERROR__"):
                enc = line[len("__WBW_WORKER_ERROR__"):]
                try:
                    message = base64.b64decode(enc).decode("utf-8", errors="replace")
                except Exception:
                    message = "unknown external runtime worker error"
                raise RuntimeBootstrapError(
                    "External whitebox_workflows runtime failed via "
                    f"{self._python_executable}: {message}"
                )
            if line.startswith("__WBW_WORKER_OK__"):
                continue
            if line.strip():
                loose_stdout_lines.append(line)

        returncode = process.poll()
        if returncode is None:
            raise RuntimeBootstrapError(
                "External runtime worker terminated unexpectedly before emitting a result."
            )

        detail = "\n".join(loose_stdout_lines).strip() or "unknown external runtime worker error"
        raise RuntimeBootstrapError(
            "External whitebox_workflows runtime failed via "
            f"{self._python_executable}: {detail}"
        )

    def _run_tool_json_stream_oneoff(self, tool_id: str, args_json: str, _callback=None) -> str:
        payload = {
            "include_pro": self._include_pro,
            "tier": self._tier,
            "tool_id": tool_id,
            "args_json": args_json,
        }

        # Stream events and final result over stdout using explicit line prefixes
        # so the caller can update QGIS progress/messages in near-real time.
        runner = (
            "import base64, json, sys\n"
            "import whitebox_workflows as wbw\n"
            "p = json.loads(sys.argv[1])\n"
            "s = wbw.RuntimeSession(include_pro=bool(p.get('include_pro', True)), tier=str(p.get('tier', 'open')))\n"
            "tool_id = str(p.get('tool_id', ''))\n"
            "args_json = str(p.get('args_json', '{}'))\n"
            "def _emit_event(evt):\n"
            "    try:\n"
            "        if isinstance(evt, str):\n"
            "            payload = evt\n"
            "        else:\n"
            "            payload = json.dumps(evt)\n"
            "    except Exception as exc:\n"
            "        payload = json.dumps({'type': 'message', 'message': str(exc)})\n"
            "    enc = base64.b64encode(payload.encode('utf-8')).decode('ascii')\n"
            "    sys.stdout.write('__WBW_EVENT__' + enc + '\\n')\n"
            "    sys.stdout.flush()\n"
            "try:\n"
            "    out = s.run_tool_json_stream(tool_id, args_json, _emit_event)\n"
            "except TypeError:\n"
            "    out = s.run_tool_json_stream(tool_id, args_json)\n"
            "except Exception:\n"
            "    out = s.run_tool_json(tool_id, args_json)\n"
            "if isinstance(out, str):\n"
            "    out_text = out\n"
            "else:\n"
            "    out_text = json.dumps(out)\n"
            "enc_out = base64.b64encode(out_text.encode('utf-8')).decode('ascii')\n"
            "sys.stdout.write('__WBW_RESULT__' + enc_out + '\\n')\n"
            "sys.stdout.flush()\n"
        )

        completed_result: str | None = None
        loose_stdout_lines: list[str] = []

        process = subprocess.Popen(
            [self._python_executable, "-c", runner, json.dumps(payload)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=self._build_clean_env(),
            bufsize=1,
        )

        assert process.stdout is not None
        for raw_line in process.stdout:
            line = raw_line.rstrip("\r\n")
            if line.startswith("__WBW_EVENT__"):
                enc = line[len("__WBW_EVENT__"):]
                try:
                    evt = base64.b64decode(enc).decode("utf-8", errors="replace")
                except Exception:
                    continue
                if callable(_callback):
                    try:
                        _callback(evt)
                    except Exception:
                        pass
                continue
            if line.startswith("__WBW_RESULT__"):
                enc = line[len("__WBW_RESULT__"):]
                try:
                    completed_result = base64.b64decode(enc).decode("utf-8", errors="replace")
                except Exception:
                    completed_result = "{}"
                continue
            if line.strip():
                loose_stdout_lines.append(line)

        stderr_text = ""
        if process.stderr is not None:
            stderr_text = process.stderr.read().strip()

        returncode = process.wait()
        if returncode != 0:
            detail = stderr_text or "\n".join(loose_stdout_lines).strip() or "unknown external runtime error"
            raise RuntimeBootstrapError(
                "External whitebox_workflows runtime failed via "
                f"{self._python_executable}: {detail}"
            )

        if completed_result is not None:
            return completed_result

        # Fallback for unexpected host/runtime output behavior.
        return "\n".join(loose_stdout_lines).strip() or "{}"

    def run_projection_wrapper_json(self, tool_id: str, args_json: str):
        return self._invoke("run_projection_wrapper_json", tool_id=tool_id, args_json=args_json)


def _discover_external_python() -> str | None:
    candidates = _discover_external_python_candidates()
    return candidates[0] if candidates else None


def _rewrite_windows_qgis_launcher(path_text: str | None) -> str:
    raw = str(path_text or "").strip()
    if os.name != "nt" or not raw:
        return raw

    normalized = raw.replace("\\", "/").lower()
    suffixes = ("/bin/python3.exe", "/bin/python.exe")
    if not any(normalized.endswith(suffix) for suffix in suffixes):
        return raw

    install_root = raw[: raw.lower().rfind("bin\\python") if "bin\\python" in raw.lower() else raw.replace("\\", "/").lower().rfind("/bin/python")]
    install_root = install_root.rstrip("\\/")
    if not install_root:
        return raw

    apps_dir = Path(install_root) / "apps"
    if not apps_dir.exists() or not apps_dir.is_dir():
        return raw

    python_dirs: list[Path] = []
    try:
        python_dirs = sorted(
            child for child in apps_dir.iterdir() if child.is_dir() and child.name.lower().startswith("python")
        )
    except Exception:
        return raw

    for python_dir in python_dirs:
        embedded = python_dir / "python.exe"
        if embedded.exists() and os.access(embedded, os.X_OK):
            return str(embedded)

    return raw


def _discover_external_python_candidates() -> list[str]:
    mode, local_python = get_runtime_preferences()
    if mode == "qgis":
        return []

    ordered: list[str] = []

    def _add_candidate(path_text: str | None) -> None:
        p_text = str(path_text or "").strip()
        if not p_text:
            return
        preferred = _rewrite_windows_qgis_launcher(p_text)
        candidate_texts = [preferred]
        if preferred != p_text:
            candidate_texts.append(p_text)

        for candidate_text in candidate_texts:
            p = Path(candidate_text).expanduser()
            if not p.exists() or not os.access(p, os.X_OK):
                continue
            value = str(p)
            if preferred != p_text and value == str(Path(p_text).expanduser()):
                continue
            if value not in ordered:
                ordered.append(value)

    if mode == "local":
        _add_candidate(local_python)
        return ordered

    env_candidate = os.environ.get("WBW_EXTERNAL_PYTHON")
    _add_candidate(env_candidate)

    default_candidates = [
        Path.home() / "Documents" / "programming" / "Rust" / "whitebox_next_gen" / ".venv-wbw" / "bin" / "python",
        Path.home() / "Documents" / "programming" / "python" / ".venv" / "bin" / "python",
        Path.home() / ".venv" / "bin" / "python",
        Path("/opt/homebrew/bin/python3"),
        Path("/opt/homebrew/bin/python"),
        Path("/usr/local/bin/python3"),
        Path("/usr/local/bin/python"),
    ]
    for candidate in default_candidates:
        _add_candidate(str(candidate))

    resolved = shutil.which("python3")
    _add_candidate(resolved)
    _add_candidate("/usr/bin/python3")

    return ordered


def load_whitebox_workflows():
    try:
        return importlib.import_module("whitebox_workflows")
    except ImportError as exc:
        raise RuntimeBootstrapError(
            "The whitebox_workflows package is not available in this Python environment. "
            "Install or activate a QGIS-compatible WbW-Py build before loading the plugin."
        ) from exc


def _effective_python_for_backend_ops() -> str | None:
    """Return the python executable used for backend package ops.

    In qgis mode, use the current in-process interpreter executable.
    In auto/local mode, use the selected external interpreter.
    """
    mode, _local_python = get_runtime_preferences()
    if mode == "qgis":
        try:
            import sys

            return str(sys.executable)
        except Exception:
            return None
    return _discover_external_python()


def get_backend_interpreter_path() -> str:
    interpreter = _effective_python_for_backend_ops()
    if not interpreter:
        raise RuntimeBootstrapError(
            "No Python interpreter is available for whitebox_workflows package operations."
        )
    return interpreter


def _get_installed_whitebox_workflows_version_for_interpreter(interpreter: str) -> str:
    runner = (
        "import importlib.metadata as md\n"
        "try:\n"
        "    print(md.version('whitebox-workflows'))\n"
        "except Exception:\n"
        "    print('')\n"
    )
    completed = subprocess.run(
        [interpreter, "-c", runner],
        check=False,
        capture_output=True,
        text=True,
        env=_build_clean_env(),
    )
    if completed.returncode != 0:
        stderr = completed.stderr.strip() or completed.stdout.strip() or "unknown version query error"
        raise RuntimeBootstrapError(
            f"Failed checking installed whitebox_workflows version via {interpreter}: {stderr}"
        )
    return completed.stdout.strip()


def get_installed_whitebox_workflows_version() -> str:
    interpreter = get_backend_interpreter_path()
    return _get_installed_whitebox_workflows_version_for_interpreter(interpreter)


def install_or_upgrade_whitebox_workflows(*, upgrade: bool = False, version_spec: str = "") -> dict:
    """Install or upgrade whitebox-workflows in the selected interpreter."""
    interpreter = get_backend_interpreter_path()
    package_spec = "whitebox-workflows"
    cleaned_spec = str(version_spec or "").strip()
    if cleaned_spec:
        package_spec = f"whitebox-workflows{cleaned_spec}"

    command = [interpreter, "-m", "pip", "install"]
    if upgrade:
        command.append("--upgrade")
    command.append(package_spec)

    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        env=_build_clean_env(),
    )
    stdout = completed.stdout.strip()
    stderr = completed.stderr.strip()

    if completed.returncode != 0:
        detail = stderr or stdout or "unknown pip install error"
        raise RuntimeBootstrapError(
            f"Failed to install {package_spec} via {interpreter}: {detail}"
        )

    installed = ""
    try:
        installed = get_installed_whitebox_workflows_version()
    except Exception:
        installed = ""

    # Existing cached sessions may point to a stale module build.
    _EXTERNAL_SESSION_CACHE.clear()

    return {
        "interpreter": interpreter,
        "installed_version": installed,
        "stdout": stdout,
        "stderr": stderr,
        "upgraded": bool(upgrade),
    }


def fetch_latest_whitebox_workflows_version(timeout_seconds: float = 4.0) -> str:
    url = "https://pypi.org/pypi/whitebox-workflows/json"
    parsed_url = urllib.parse.urlparse(url)
    if parsed_url.scheme != "https" or parsed_url.netloc != "pypi.org":
        raise RuntimeBootstrapError(
            "Refusing to query package index from non-HTTPS or untrusted host."
        )

    try:
        # URL is validated above (scheme + host allowlist).
        with urllib.request.urlopen(url, timeout=float(timeout_seconds)) as response:  # nosec B310
            payload = response.read().decode("utf-8", errors="replace")
        parsed = json.loads(payload)
        info = parsed.get("info", {}) if isinstance(parsed, dict) else {}
        version = str(info.get("version", "")).strip()
        return version
    except Exception as exc:
        raise RuntimeBootstrapError(f"Unable to query latest whitebox-workflows version from PyPI: {exc}")


def _normalize_version_for_compare(raw: str) -> tuple:
    text = str(raw or "").strip().lower()
    if not text:
        return tuple()
    parts = []
    token = ""
    for ch in text:
        if ch.isalnum():
            token += ch
            continue
        if token:
            parts.append(token)
            token = ""
    if token:
        parts.append(token)

    normalized = []
    for part in parts:
        if part.isdigit():
            normalized.append((0, int(part)))
        else:
            normalized.append((1, part))
    return tuple(normalized)


def is_version_newer(candidate: str, current: str) -> bool:
    return _normalize_version_for_compare(candidate) > _normalize_version_for_compare(current)


def _ensure_next_gen_min_version(installed_version: str, runtime_label: str) -> None:
    version_text = str(installed_version or "").strip()
    if not version_text:
        return
    if _normalize_version_for_compare(version_text) < _normalize_version_for_compare(_NEXT_GEN_MIN_VERSION):
        raise RuntimeBootstrapError(
            _next_gen_required_message(
                f"{runtime_label} has whitebox_workflows {version_text}; required >= {_NEXT_GEN_MIN_VERSION}."
            )
        )


def backend_update_status() -> dict:
    """Return installed/latest version status for whitebox-workflows.

    This function never raises; failures are captured in an error field.
    """
    result = {
        "interpreter": "",
        "installed_version": "",
        "latest_version": "",
        "update_available": False,
        "error": "",
    }
    try:
        result["interpreter"] = get_backend_interpreter_path()
        result["installed_version"] = get_installed_whitebox_workflows_version()
        result["latest_version"] = fetch_latest_whitebox_workflows_version()
        installed = str(result.get("installed_version", "")).strip()
        latest = str(result.get("latest_version", "")).strip()
        if installed and latest:
            result["update_available"] = is_version_newer(latest, installed)
        elif latest and not installed:
            result["update_available"] = True
    except Exception as exc:
        result["error"] = str(exc)
    return result


def backend_install_status() -> dict:
    """Return local installation status without querying remote package indexes."""
    result = {
        "interpreter": "",
        "installed_version": "",
        "error": "",
    }
    try:
        result["interpreter"] = get_backend_interpreter_path()
        result["installed_version"] = get_installed_whitebox_workflows_version()
    except Exception as exc:
        result["error"] = str(exc)
    return result


def _build_clean_env() -> dict[str, str]:
    """Build a clean environment for subprocess calls (remove PYTHONHOME, etc.)."""
    clean_env = dict(os.environ)
    clean_env.pop("PYTHONHOME", None)
    clean_env.pop("PYTHONPATH", None)
    return clean_env


def invoke_license_function(
    function_name: str,
    key: str | None = None,
    firstname: str | None = None,
    lastname: str | None = None,
    email: str | None = None,
    provider_url: str | None = None,
    from_transfer: bool = False,
) -> str:
    """
    Invoke a license function (activate_license, deactivate_license, transfer_license)
    via the discovered external Python interpreter.

    This ensures license operations use the same Python environment as tool execution,
    avoiding split-brain where the in-process QGIS Python might be stale/different.

    Args:
        function_name: 'activate_license', 'deactivate_license', or 'transfer_license'
        key, firstname, lastname, email, provider_url: parameters for activate_license
        from_transfer: parameter for deactivate_license

    Returns:
        String result from the license function.

    Raises:
        RuntimeBootstrapError on execution or availability failure.
    """
    external_python = _discover_external_python()
    if not external_python:
        raise RuntimeBootstrapError(
            "No external Python interpreter was found for license operations."
        )

    if function_name == "activate_license":
        if not all([key, firstname, lastname, email]):
            raise RuntimeBootstrapError(
                "activate_license requires: key, firstname, lastname, email"
            )
        runner = (
            "import json, sys\n"
            "import whitebox_workflows as wbw\n"
            "params = json.loads(sys.argv[1])\n"
            "result = wbw.activate_license(\n"
            "    key=str(params['key']),\n"
            "    firstname=str(params['firstname']),\n"
            "    lastname=str(params['lastname']),\n"
            "    email=str(params['email']),\n"
            "    agree_to_license_terms=True,\n"
            "    provider_url=params.get('provider_url') or None,\n"
            "    include_pro=True,\n"
            ")\n"
            "sys.stdout.write(str(result))\n"
        )
        payload = {
            "key": key,
            "firstname": firstname,
            "lastname": lastname,
            "email": email,
            "provider_url": provider_url,
        }
    elif function_name == "transfer_license":
        runner = (
            "import sys\n"
            "import whitebox_workflows as wbw\n"
            "result = wbw.transfer_license()\n"
            "sys.stdout.write(str(result))\n"
        )
        payload = {}
    elif function_name == "deactivate_license":
        runner = (
            "import json, sys\n"
            "import whitebox_workflows as wbw\n"
            "params = json.loads(sys.argv[1])\n"
            "result = wbw.deactivate_license(from_transfer=bool(params.get('from_transfer', False)))\n"
            "sys.stdout.write(str(result))\n"
        )
        payload = {"from_transfer": from_transfer}
    else:
        raise RuntimeBootstrapError(
            f"Unknown license function: {function_name}"
        )

    completed = subprocess.run(
        [external_python, "-c", runner, json.dumps(payload)] if payload else [external_python, "-c", runner],
        check=False,
        capture_output=True,
        text=True,
        env=_build_clean_env(),
    )

    if completed.returncode != 0:
        stderr = completed.stderr.strip() or completed.stdout.strip() or "unknown license operation error"
        raise RuntimeBootstrapError(
            f"License operation '{function_name}' failed via {external_python}: {stderr}"
        )

    return completed.stdout.strip()


def create_runtime_session(include_pro: bool = True, tier: str = "open"):
    def _external_session(prefer_pro: bool, allow_downgrade: bool = True):
        external_candidates = _discover_external_python_candidates()
        if not external_candidates:
            raise RuntimeBootstrapError(
                "No external Python interpreter was found for whitebox_workflows fallback."
            )

        failures: list[str] = []
        for external_python in external_candidates:
            try:
                detected_version = _get_installed_whitebox_workflows_version_for_interpreter(external_python)
                _ensure_next_gen_min_version(detected_version, f"external runtime ({external_python})")

                cache_key = (external_python, bool(prefer_pro), str(tier))
                cached = _EXTERNAL_SESSION_CACHE.get(cache_key)
                if cached is not None:
                    return cached

                session = ExternalRuntimeSession(external_python, include_pro=prefer_pro, tier=tier)
                raw_caps = session.get_runtime_capabilities_json()
                _parse_next_gen_capabilities(raw_caps, f"external runtime ({external_python})")
                _EXTERNAL_SESSION_CACHE[cache_key] = session
                return session
            except RuntimeBootstrapError as exc:
                if allow_downgrade and prefer_pro and _is_pro_unavailable_error(str(exc)):
                    try:
                        downgraded = ExternalRuntimeSession(external_python, include_pro=False, tier=tier)
                        raw_caps = downgraded.get_runtime_capabilities_json()
                        _parse_next_gen_capabilities(raw_caps, f"external runtime ({external_python})")
                        downgraded_key = (external_python, False, str(tier))
                        _EXTERNAL_SESSION_CACHE[downgraded_key] = downgraded
                        return downgraded
                    except RuntimeBootstrapError as downgrade_exc:
                        failures.append(f"{external_python}: {downgrade_exc}")
                        continue
                failures.append(f"{external_python}: {exc}")
                continue

        detail = " | ".join(failures) if failures else "unknown external runtime initialization error"
        raise RuntimeBootstrapError(
            "Unable to initialize whitebox_workflows Next Gen runtime from discovered external interpreters. "
            f"Tried: {', '.join(external_candidates)}. Detail: {detail}"
        )

    # Prefer a discovered external runtime first. In environments where QGIS
    # ships with a different embedded Python package set, this avoids silently
    # binding to a stale in-process whitebox_workflows install.
    try:
        return _external_session(prefer_pro=include_pro, allow_downgrade=True)
    except RuntimeBootstrapError:
        # Fall back to the current Python runtime if an external runtime is not
        # available or cannot provide a valid Next Gen session.
        pass

    try:
        wbw = load_whitebox_workflows()
        try:
            import sys

            detected_version = _get_installed_whitebox_workflows_version_for_interpreter(str(sys.executable))
            _ensure_next_gen_min_version(detected_version, "current Python runtime")
        except RuntimeBootstrapError:
            raise
        except Exception:
            # If version metadata is unavailable (e.g., editable/dev context),
            # keep the existing capabilities-based gate below.
            pass
        if not hasattr(wbw, "RuntimeSession"):
            try:
                return _external_session(prefer_pro=include_pro, allow_downgrade=True)
            except Exception:
                raise RuntimeBootstrapError(_next_gen_required_message("RuntimeSession class not found."))
        try:
            session = wbw.RuntimeSession(include_pro=include_pro, tier=tier)
            raw_caps = session.get_runtime_capabilities_json()
            _parse_next_gen_capabilities(raw_caps, "current Python runtime")
            return session
        except Exception as exc:
            if include_pro and _is_pro_unavailable_error(str(exc)):
                try:
                    downgraded = wbw.RuntimeSession(include_pro=False, tier=tier)
                    raw_caps = downgraded.get_runtime_capabilities_json()
                    _parse_next_gen_capabilities(raw_caps, "current Python runtime")
                    return downgraded
                except Exception:
                    # Only fall back to a different external interpreter if the
                    # current Python runtime cannot even provide a valid OSS-only
                    # Next Gen session. This avoids silently switching to a stale
                    # external environment with a different tool catalog.
                    return _external_session(prefer_pro=True, allow_downgrade=True)
            if _is_legacy_runtime_error(str(exc)):
                try:
                    return _external_session(prefer_pro=include_pro, allow_downgrade=True)
                except Exception:
                    raise RuntimeBootstrapError(_next_gen_required_message(str(exc))) from exc
            raise
    except RuntimeBootstrapError as exc:
        if _is_legacy_runtime_error(str(exc)):
            raise
        return _external_session(prefer_pro=include_pro, allow_downgrade=True)


def get_runtime_capabilities(include_pro: bool = True, tier: str = "open") -> dict:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    return json.loads(session.get_runtime_capabilities_json())


def get_tool_catalog(include_pro: bool = True, tier: str = "open") -> list[dict]:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    return json.loads(session.list_tool_catalog_json())


def get_tool_metadata(tool_id: str, include_pro: bool = True, tier: str = "open") -> dict:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    return json.loads(session.get_tool_metadata_json(tool_id))


def run_projection_wrapper(
    tool_id: str,
    args: dict,
    *,
    include_pro: bool = True,
    tier: str = "open",
) -> dict:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    method = getattr(session, "run_projection_wrapper_json", None)
    if callable(method):
        raw = method(tool_id, json.dumps(args))
        return json.loads(raw)

    wbw = load_whitebox_workflows()
    wbe = wbw.WbEnvironment()
    epsg = int(args.get("epsg"))
    in_path = str(args.get("input", ""))
    out_path = str(args.get("output", ""))

    if tool_id == "reproject_raster":
        resample = str(args.get("resample", "bilinear")).strip() or "bilinear"
        src = wbe.read_raster(in_path)
        out_obj = wbe.reproject_raster(src, dst_epsg=epsg, resample=resample)
        wbe.write_raster(out_obj, out_path)
    elif tool_id == "georeference_raster_from_control_points":
        resample = str(args.get("resample", "bilinear")).strip() or "bilinear"
        src = wbe.read_raster(in_path)
        control_points = args.get("control_points")
        out_obj = wbe.georeference_raster_from_control_points(
            src,
            control_points=control_points,
            epsg=epsg,
            output=out_path or None,
            report=str(args.get("report", "")).strip() or None,
            resample=resample,
        )
    elif tool_id == "reproject_lidar":
        src = wbe.read_lidar(in_path)
        out_obj = wbe.reproject_lidar(src, dst_epsg=epsg)
        wbe.write_lidar(out_obj, out_path)
    elif tool_id == "assign_projection_raster":
        src = wbe.read_raster(in_path)
        src.set_crs_epsg(epsg)
        if not out_path:
            out_path = in_path
        wbe.write_raster(src, out_path)
    elif tool_id == "assign_projection_vector":
        src = wbe.read_vector(in_path)
        src.set_crs_epsg(epsg)
        if not out_path:
            out_path = in_path
        wbe.write_vector(src, out_path)
    elif tool_id == "assign_projection_lidar":
        src = wbe.read_lidar(in_path)
        src.set_crs_epsg(epsg)
        if not out_path:
            out_path = in_path
        wbe.write_lidar(src, out_path)
    else:
        raise RuntimeBootstrapError(f"Unsupported projection wrapper tool: {tool_id}")

    return {"outputs": {"output": out_path or in_path}}
