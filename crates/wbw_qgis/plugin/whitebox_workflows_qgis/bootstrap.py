from __future__ import annotations

import base64
import importlib
import importlib.util
import json
import os
import shutil
import subprocess
import urllib.parse
import urllib.request
from pathlib import Path


_EXTERNAL_SESSION_CACHE: dict[tuple[str, bool, str], "ExternalRuntimeSession"] = {}
_RUNTIME_MODE = "qgis"
_RUNTIME_LOCAL_PYTHON = ""
_NEXT_GEN_MIN_VERSION = "2.0.0"
_WBW_PIP_TARGET_DIR = ""  # cached pip --target directory


def _subprocess_window_kwargs() -> dict:
    """Return platform-safe subprocess kwargs that avoid transient console windows."""
    if os.name != "nt":
        return {}

    kwargs: dict[str, object] = {}
    creationflags = 0
    create_no_window = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    if create_no_window:
        creationflags |= create_no_window
    if creationflags:
        kwargs["creationflags"] = creationflags

    startupinfo_cls = getattr(subprocess, "STARTUPINFO", None)
    use_show_window = getattr(subprocess, "STARTF_USESHOWWINDOW", 0)
    sw_hide = getattr(subprocess, "SW_HIDE", 0)
    if startupinfo_cls is not None and use_show_window:
        startupinfo = startupinfo_cls()
        startupinfo.dwFlags |= use_show_window
        startupinfo.wShowWindow = sw_hide
        kwargs["startupinfo"] = startupinfo

    return kwargs


def _get_site_packages_for_python(python_exe: str | None) -> str | None:
    """Given a Python executable path, return its site-packages directory.

    Returns the first writable site-packages directory found, or None if
    the path is invalid, not executable, or site-packages cannot be determined.
    """
    exe_path = str(python_exe or "").strip()
    if not exe_path:
        return None

    try:
        path_obj = Path(exe_path)
        if not path_obj.exists() or not os.access(path_obj, os.X_OK):
            return None
    except Exception:
        return None

    try:
        result = subprocess.run(
            [exe_path, "-c", "import site; print('\\n'.join(site.getsitepackages()))"],
            capture_output=True,
            text=True,
            timeout=5,
            **_subprocess_window_kwargs(),
        )
        if result.returncode == 0:
            lines = result.stdout.strip().split('\n')
            for line in lines:
                line = line.strip()
                if line and os.path.isdir(line):
                    return line
    except Exception:
        pass

    return None


def set_runtime_preferences(mode: str = "qgis", local_python: str = "") -> None:
    """Set runtime interpreter selection preferences for this plugin process.

    mode values:
    - qgis: use in-process QGIS Python runtime (default, recommended for end users)
    - local: use a caller-provided local interpreter path (for development)
    """
    global _RUNTIME_MODE, _RUNTIME_LOCAL_PYTHON
    normalized = str(mode or "qgis").strip().lower()
    if normalized not in {"local", "qgis"}:
        normalized = "qgis"
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
    return all(
        marker in text
        for marker in (
            "include_pro=true requested",
            "does not include pro support",
        )
    )


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
            "            out = wbw.run_tool_json_stream_options(tool_id, args_json, _emit_event, include_pro, tier)\n"
            "        except TypeError:\n"
            "            out = wbw.run_tool_json_with_progress_options(tool_id, args_json, include_pro, tier)\n"
            "        except Exception:\n"
            "            out = wbw.run_tool_json_with_options(tool_id, args_json, include_pro, tier)\n"
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
            **_subprocess_window_kwargs(),
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
        finally:
            try:
                if process.stdin is not None:
                    process.stdin.close()
            except Exception:
                pass
            try:
                if process.stdout is not None:
                    process.stdout.close()
            except Exception:
                pass
            try:
                if process.stderr is not None:
                    process.stderr.close()
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
            "include_pro = bool(p.get('include_pro', True))\n"
            "tier = str(p.get('tier', 'open'))\n"
            "m = p.get('method')\n"
            "if m == 'get_runtime_capabilities_json':\n"
            "    out = wbw.get_runtime_capabilities_json_with_options(include_pro, tier)\n"
            "elif m == 'list_tool_catalog_json':\n"
            "    out = wbw.list_tool_catalog_json_with_options(include_pro, tier)\n"
            "elif m == 'get_tool_metadata_json':\n"
            "    out = wbw.get_tool_metadata_json_with_options(str(p.get('tool_id', '')), include_pro, tier)\n"
            "elif m == 'run_tool_json_stream':\n"
            "    tool_id = str(p.get('tool_id', ''))\n"
            "    args_json = str(p.get('args_json', '{}'))\n"
            "    try:\n"
            "        out = wbw.run_tool_json_stream_options(tool_id, args_json, lambda _evt: None, include_pro, tier)\n"
            "    except TypeError:\n"
            "        out = wbw.run_tool_json_with_progress_options(tool_id, args_json, include_pro, tier)\n"
            "    except Exception:\n"
            "        out = wbw.run_tool_json_with_options(tool_id, args_json, include_pro, tier)\n"
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
            **_subprocess_window_kwargs(),
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
            "include_pro = bool(p.get('include_pro', True))\n"
            "tier = str(p.get('tier', 'open'))\n"
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
            "    out = wbw.run_tool_json_stream_options(tool_id, args_json, _emit_event, include_pro, tier)\n"
            "except TypeError:\n"
            "    out = wbw.run_tool_json_with_progress_options(tool_id, args_json, include_pro, tier)\n"
            "except Exception:\n"
            "    out = wbw.run_tool_json_with_options(tool_id, args_json, include_pro, tier)\n"
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

        with subprocess.Popen(
            [self._python_executable, "-c", runner, json.dumps(payload)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=self._build_clean_env(),
            bufsize=1,
            **_subprocess_window_kwargs(),
        ) as process:
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


def _is_probably_python_executable(path_text: str | None) -> bool:
    text = str(path_text or "").strip()
    if not text:
        return False
    try:
        name = Path(text).name.lower()
    except Exception:
        return False
    if os.name == "nt":
        return name.startswith("python") and name.endswith(".exe")
    return name.startswith("python")


def _discover_embedded_qgis_python(path_text: str | None) -> str | None:
    raw = str(path_text or "").strip()
    if not raw:
        return None

    try:
        exe_path = Path(raw)
    except Exception:
        return None

    if not exe_path.exists():
        return None

    if os.name == "nt":
        # Windows: C:\Program Files\QGIS <ver>\bin\qgis-bin.exe
        #       -> ..\apps\Python<xy>\python.exe
        parent = exe_path.parent
        if parent.name.lower() != "bin":
            return None

        install_root = parent.parent
        apps_dir = install_root / "apps"
        if not apps_dir.exists() or not apps_dir.is_dir():
            return None

        try:
            python_dirs = sorted(
                child for child in apps_dir.iterdir() if child.is_dir() and child.name.lower().startswith("python")
            )
        except Exception:
            return None

        for python_dir in python_dirs:
            embedded = python_dir / "python.exe"
            if embedded.exists() and os.access(embedded, os.X_OK):
                return str(embedded)

        return None

    # macOS: QGIS.app/Contents/MacOS/QGIS
    #     -> QGIS.app/Contents/MacOS/python3  (some builds)
    #     -> QGIS.app/Contents/Frameworks/Python.framework/.../python3
    try:
        macos_dir = exe_path.parent  # .../QGIS.app/Contents/MacOS/
        # Candidate 1: python3 / python alongside the QGIS binary
        for name in ("python3", "python"):
            candidate = macos_dir / name
            if candidate.exists() and os.access(candidate, os.X_OK):
                return str(candidate)

        # Candidate 2: Python.framework inside Contents/Frameworks
        frameworks_dir = macos_dir.parent / "Frameworks"
        if frameworks_dir.is_dir():
            fw = frameworks_dir / "Python.framework" / "Versions"
            if fw.is_dir():
                versions = sorted(fw.iterdir(), reverse=True)
                for ver in versions:
                    for name in ("bin/python3", "bin/python"):
                        candidate = ver / name
                        if candidate.exists() and os.access(candidate, os.X_OK):
                            return str(candidate)
    except Exception:
        pass

    return None


def _rewrite_windows_qgis_launcher(path_text: str | None) -> str:
    raw = str(path_text or "").strip()
    if os.name != "nt" or not raw:
        return raw

    normalized = raw.replace("\\", "/").lower()
    suffixes = ("/bin/python3.exe", "/bin/python.exe")
    if not any(normalized.endswith(suffix) for suffix in suffixes):
        return raw

    install_root = raw[
        : raw.lower().rfind("bin\\python")
        if "bin\\python" in raw.lower()
        else raw.replace("\\", "/").lower().rfind("/bin/python")
    ]
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


def _get_pip_target_dir() -> str:
    """Return a writable per-profile directory for pip --target installs.

    Uses the QGIS profile's python directory when available so that the
    installed package persists across QGIS sessions and is isolated from
    the (read-only) QGIS app bundle site-packages.
    """
    global _WBW_PIP_TARGET_DIR
    if _WBW_PIP_TARGET_DIR:
        return _WBW_PIP_TARGET_DIR

    target: Path | None = None

    # Prefer the QGIS profile directory - available when running inside QGIS.
    try:
        from qgis.core import QgsApplication  # type: ignore[import]
        config_path = QgsApplication.qgisSettingsDirPath()
        if config_path:
            target = Path(config_path) / "python" / "whitebox_workflows_lib"
    except Exception:
        pass

    # Fallback: user home directory.
    if target is None:
        target = Path.home() / ".whitebox_workflows_lib"

    target.mkdir(parents=True, exist_ok=True)
    _WBW_PIP_TARGET_DIR = str(target)
    return _WBW_PIP_TARGET_DIR


def _ensure_pip_target_on_sys_path() -> None:
    """Add the pip --target directory to sys.path if not already present."""
    import sys
    target = _get_pip_target_dir()
    if target and target not in sys.path:
        sys.path.insert(0, target)


def _install_whitebox_workflows_via_pip(target_dir: str) -> None:
    """Install whitebox-workflows into *target_dir* using the QGIS Python pip.

    Uses sys.executable (the QGIS Python) and preserves the full process
    environment so that PYTHONHOME is correctly inherited on macOS/Linux
    bundled distributions.
    """
    import sys
    completed = subprocess.run(
        [
            sys.executable, "-m", "pip", "install",
            "--target", target_dir,
            "--upgrade",
            "whitebox-workflows",
        ],
        check=False,
        capture_output=True,
        text=True,
        env=dict(os.environ),  # keep full env - PYTHONHOME required for bundled Python
        **_subprocess_window_kwargs(),
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "unknown pip error"
        raise RuntimeBootstrapError(
            f"pip install whitebox-workflows failed: {detail}"
        )


def load_whitebox_workflows():
    import sys

    # When switching runtime modes, clear the cached module so we reimport from the new location.
    # Otherwise Python returns the cached module and importlib caches have no effect.
    mode, local_python = get_runtime_preferences()
    if mode == "local":
        # Remove cached whitebox_workflows and all its submodules
        sys.modules.pop("whitebox_workflows", None)
        to_remove = [key for key in sys.modules.keys() if key.startswith("whitebox_workflows.")]
        for key in to_remove:
            sys.modules.pop(key, None)
        # Clear Python's import machinery cache (PathFinder, etc.) so it rescans for modules
        importlib.invalidate_caches()

    # For local mode: directly load from the development path using importlib
    # This avoids permanently modifying sys.path and breaking QGIS plugin compatibility
    if mode == "local" and local_python:
        site_packages = _get_site_packages_for_python(local_python)
        if site_packages:
            # Look for .pth files that might point to development directories (from maturin develop)
            pth_files = list(Path(site_packages).glob("whitebox_workflows*.pth"))
            for pth_file in pth_files:
                try:
                    pth_path = pth_file.read_text().strip()
                    if pth_path and Path(pth_path).exists():
                        # Try to load from the .pth path directly
                        wbw_module_path = Path(pth_path) / "whitebox_workflows" / "__init__.py"
                        if wbw_module_path.exists():
                            spec = importlib.util.spec_from_file_location(
                                "whitebox_workflows",
                                wbw_module_path
                            )
                            if spec and spec.loader:
                                wbw = importlib.util.module_from_spec(spec)
                                sys.modules["whitebox_workflows"] = wbw
                                spec.loader.exec_module(wbw)
                                return wbw
                except Exception:
                    pass

            # If .pth approach didn't work, try site-packages itself
            wbw_module_path = Path(site_packages) / "whitebox_workflows" / "__init__.py"
            if wbw_module_path.exists():
                try:
                    spec = importlib.util.spec_from_file_location(
                        "whitebox_workflows",
                        wbw_module_path
                    )
                    if spec and spec.loader:
                        wbw = importlib.util.module_from_spec(spec)
                        sys.modules["whitebox_workflows"] = wbw
                        spec.loader.exec_module(wbw)
                        return wbw
                except Exception:
                    pass

    # Attempt 1: direct import (standard Python path resolution for QGIS Python)
    try:
        return importlib.import_module("whitebox_workflows")
    except ImportError:
        pass

    # Attempt 2: add the pip target dir to sys.path and retry.
    # Covers the case where a previous install ran but the dir wasn't on sys.path.
    _ensure_pip_target_on_sys_path()
    try:
        return importlib.import_module("whitebox_workflows")
    except ImportError:
        pass

    # Not installed. Raise a specific sentinel so callers can offer to install.
    raise RuntimeBootstrapError(
        "BACKEND_NOT_INSTALLED: whitebox-workflows is not installed. "
        "The plugin can install it automatically — please use "
        "Plugins → Whitebox Workflows → Plugin Settings to install the backend."
    )


def is_backend_not_installed_error(exc: Exception) -> bool:
    """Return True if *exc* is the specific 'backend not installed' sentinel."""
    return str(exc).startswith("BACKEND_NOT_INSTALLED:")


def get_loaded_backend_info() -> dict:
    """Return diagnostic info about the currently loaded whitebox_workflows.

    Returns a dict with:
    - version: version string (e.g., "2.0.3")
    - file_path: location of the whitebox_workflows package
    - mode: current runtime mode ("qgis" or "local")
    - local_python_path: path to local Python if in local mode, else ""
    """
    try:
        wbw = importlib.import_module("whitebox_workflows")

        version = "unknown"
        try:
            version = importlib.metadata.version("whitebox_workflows")
        except Exception:
            if hasattr(wbw, "__version__"):
                version = str(wbw.__version__)

        file_path = "unknown"
        if hasattr(wbw, "__file__") and wbw.__file__:
            file_path = str(wbw.__file__)

        mode, local_python = get_runtime_preferences()

        return {
            "version": version,
            "file_path": file_path,
            "mode": mode,
            "local_python_path": local_python if mode == "local" else "",
        }
    except Exception as exc:
        return {
            "version": "error",
            "file_path": str(exc),
            "mode": "",
            "local_python_path": "",
        }


def _effective_python_for_backend_ops() -> str | None:
    """Return the python executable used for backend package ops.

    In qgis mode, use the current in-process interpreter executable.
    In auto/local mode, use the selected external interpreter.

    On macOS (and some Linux builds) sys.executable inside QGIS is the QGIS
    binary, not Python.  We try several discovery paths in order:
      1. Rewrite Windows launcher paths to the real python.exe (Windows only).
      2. Discover an embedded Python alongside the QGIS binary (Windows + macOS).
      3. Accept sys.executable if it looks like Python directly.
      4. Fall back to the in-process sentinel (sys.executable as-is) when
         whitebox_workflows is importable in the current process — the caller
         _get_installed_whitebox_workflows_version_for_interpreter has a fast
         in-process path that never spawns a subprocess, so a non-Python
         sys.executable is harmless in that code path.
    """
    mode, _local_python = get_runtime_preferences()
    if mode == "qgis":
        try:
            import sys

            current = str(sys.executable)
            rewritten = _rewrite_windows_qgis_launcher(current)
            if _is_probably_python_executable(rewritten):
                return rewritten

            embedded = _discover_embedded_qgis_python(current)
            if _is_probably_python_executable(embedded):
                return embedded

            if _is_probably_python_executable(current):
                return current

            # sys.executable is the QGIS binary (common on macOS / some Linux
            # builds).  If whitebox_workflows is already importable in-process,
            # return current as a sentinel — callers that only need version
            # metadata will use the fast in-process importlib path and never
            # actually spawn sys.executable as a subprocess.
            try:
                import importlib
                importlib.import_module("whitebox_workflows")
                return current
            except ImportError:
                pass

            return None
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
    """Get the installed whitebox-workflows version.

    For the in-process (QGIS Python) case we query importlib.metadata directly —
    no subprocess. sys.executable inside QGIS is the QGIS binary, not Python,
    so subprocess.run([sys.executable, ...]) would launch QGIS, not Python.
    """
    import sys as _sys
    is_inprocess = (interpreter == str(_sys.executable))

    if is_inprocess:
        # Fast in-process path: no subprocess needed.
        _ensure_pip_target_on_sys_path()
        try:
            import importlib.metadata as _md
            return _md.version("whitebox-workflows")
        except Exception:
            return ""

    # External interpreter path (unchanged from original).
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
        **_subprocess_window_kwargs(),
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
    """Install or upgrade whitebox-workflows using pip in-process.

    Runs pip via its internal Python API (no subprocess) so that:
    - sys.executable does not need to be a Python binary (it isn't in QGIS)
    - No stdout/stderr capture issues inside QGIS
    - No hang from subprocess waiting for QGIS to exit

    Installs into the per-profile pip --target directory.
    NOTE: Blocks until pip completes. Call via install_whitebox_workflows_async
    from the UI to avoid freezing Qt.
    """
    target_dir = _get_pip_target_dir()
    package_spec = "whitebox-workflows"
    cleaned_spec = str(version_spec or "").strip()
    if cleaned_spec:
        package_spec = f"whitebox-workflows{cleaned_spec}"

    args = ["install", "--target", target_dir]
    if upgrade:
        args.append("--upgrade")
    args.append(package_spec)

    try:
        from pip._internal.cli.main import main as pip_main  # type: ignore[import]
    except ImportError:
        raise RuntimeBootstrapError(
            "pip is not available in this Python environment. "
            "Install whitebox-workflows manually and restart QGIS."
        )

    import io  # noqa: F401 — imported for future use if pip output capture is needed
    exit_code = 1
    try:
        # pip calls sys.exit() on completion; catch it.
        exit_code = pip_main(args)
    except SystemExit as exc:
        exit_code = int(exc.code) if exc.code is not None else 0
    except Exception as exc:
        raise RuntimeBootstrapError(f"pip install failed: {exc}") from exc

    if exit_code not in (0, None):
        raise RuntimeBootstrapError(
            f"pip install {package_spec} failed with exit code {exit_code}. "
            "Check the QGIS Python console for details."
        )

    # Add target dir to sys.path and clear stale import cache.
    _ensure_pip_target_on_sys_path()
    import sys as _sys
    for key in list(_sys.modules.keys()):
        if key == "whitebox_workflows" or key.startswith("whitebox_workflows."):
            del _sys.modules[key]
    _EXTERNAL_SESSION_CACHE.clear()

    installed = ""
    try:
        installed = _get_installed_whitebox_workflows_version_for_interpreter(str(_sys.executable))
    except Exception:
        installed = ""

    import sys
    return {
        "interpreter": str(sys.executable),
        "target_dir": target_dir,
        "installed_version": installed,
        "upgraded": bool(upgrade),
    }


def install_whitebox_workflows_async(
    on_success,   # callable(result_dict)
    on_error,     # callable(error_message: str)
    *,
    upgrade: bool = False,
) -> None:
    """Run install_or_upgrade_whitebox_workflows in a background thread.

    *on_success* and *on_error* are called from the background thread.
    If you need to update Qt widgets, use a signal/slot or QMetaObject.invokeMethod
    in those callbacks.
    """
    import threading

    def _worker():
        try:
            result = install_or_upgrade_whitebox_workflows(upgrade=upgrade)
            try:
                on_success(result)
            except Exception:
                pass
        except Exception as exc:
            try:
                on_error(str(exc))
            except Exception:
                pass

    thread = threading.Thread(target=_worker, daemon=True)
    thread.start()


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
    """Return local installation status using the QGIS Python (sys.executable).

    installed_version is only populated if the version meets the minimum
    Next Gen requirement (>= 2.0.0). A legacy v0.x or v1.x package is
    treated as absent so the user is prompted to install the correct version.
    """
    import sys
    result = {
        "interpreter": str(sys.executable),
        "installed_version": "",
        "error": "",
    }
    try:
        _ensure_pip_target_on_sys_path()
        version = _get_installed_whitebox_workflows_version_for_interpreter(str(sys.executable))
        if version and _normalize_version_for_compare(version) >= _normalize_version_for_compare(_NEXT_GEN_MIN_VERSION):
            result["installed_version"] = version
        elif version:
            result["error"] = (
                f"Found whitebox_workflows {version} but require >= {_NEXT_GEN_MIN_VERSION}. "
                "The plugin will reinstall the correct version."
            )
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
        # No external Python configured (standard QGIS-mode installation).
        # Call the license functions in-process via the already-loaded
        # whitebox_workflows module rather than spawning a subprocess.
        try:
            wbw = load_whitebox_workflows()
        except Exception as exc:
            raise RuntimeBootstrapError(
                f"whitebox_workflows is not available for in-process license operations: {exc}"
            )
        try:
            if function_name == "activate_license":
                if not all([key, firstname, lastname, email]):
                    raise RuntimeBootstrapError(
                        "activate_license requires: key, firstname, lastname, email"
                    )
                result = wbw.activate_license(
                    key=str(key),
                    firstname=str(firstname),
                    lastname=str(lastname),
                    email=str(email),
                    agree_to_license_terms=True,
                    provider_url=provider_url or None,
                    include_pro=True,
                )
                return str(result)
            elif function_name == "transfer_license":
                result = wbw.transfer_license()
                return str(result)
            elif function_name == "deactivate_license":
                result = wbw.deactivate_license(from_transfer=bool(from_transfer))
                return str(result)
            else:
                raise RuntimeBootstrapError(f"Unknown license function: {function_name}")
        except RuntimeBootstrapError:
            raise
        except Exception as exc:
            raise RuntimeBootstrapError(
                f"License operation '{function_name}' failed in-process: {exc}"
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
        **_subprocess_window_kwargs(),
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

    # Prefer in-process first: load_whitebox_workflows() will auto-install via
    # the QGIS Python pip if whitebox_workflows is not yet present. This is the
    # correct default because the QGIS Python IS the execution environment and
    # always has pip available (bundled since QGIS 3.x).
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
                    # Cannot downgrade to open mode; re-raise the original error
                    raise
            if _is_legacy_runtime_error(str(exc)):
                raise RuntimeBootstrapError(_next_gen_required_message(str(exc))) from exc
            raise
    except RuntimeBootstrapError:
        # QGIS-only mode: don't try external Python fallback
        # Just re-raise the error (e.g., BACKEND_NOT_INSTALLED)
        raise


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
