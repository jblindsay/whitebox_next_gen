from __future__ import annotations

import base64
import glob
import importlib
import json
import os
import shutil
import ssl
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path


_EXTERNAL_SESSION_CACHE: dict[tuple[str, bool, str], "ExternalRuntimeSession"] = {}
_RUNTIME_MODE = "auto"
_RUNTIME_LOCAL_PYTHON = ""
_NEXT_GEN_MIN_VERSION = "2.0.0"


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
            "    def _to_optional_bool(v):\n"
            "        if v is None:\n"
            "            return None\n"
            "        if isinstance(v, bool):\n"
            "            return v\n"
            "        if isinstance(v, str):\n"
            "            t = v.strip().lower()\n"
            "            if t in ('true', '1', 'yes', 'y', 'on'):\n"
            "                return True\n"
            "            if t in ('false', '0', 'no', 'n', 'off'):\n"
            "                return False\n"
            "            return None\n"
            "        return bool(v)\n"
            "    epsg = int(args.get('epsg'))\n"
            "    in_path = str(args.get('input', ''))\n"
            "    out_path = str(args.get('output', ''))\n"
            "    coordinate_epoch = args.get('coordinate_epoch', None)\n"
            "    source_reference_epoch = args.get('source_reference_epoch', None)\n"
            "    target_reference_epoch = args.get('target_reference_epoch', None)\n"
            "    operation_code = args.get('operation_code', None)\n"
            "    prefer_official_operation = _to_optional_bool(args.get('prefer_official_operation', None))\n"
            "    epoch_policy = args.get('epoch_policy', None)\n"
            "    if tool_id == 'reproject_raster':\n"
            "        resample = str(args.get('resample', 'bilinear')).strip() or 'bilinear'\n"
            "        src = wbe.read_raster(in_path)\n"
            "        kwargs = {'dst_epsg': epsg, 'resample': resample}\n"
            "        if coordinate_epoch is not None:\n"
            "            kwargs['coordinate_epoch'] = float(coordinate_epoch)\n"
            "        if source_reference_epoch is not None:\n"
            "            kwargs['source_reference_epoch'] = float(source_reference_epoch)\n"
            "        if target_reference_epoch is not None:\n"
            "            kwargs['target_reference_epoch'] = float(target_reference_epoch)\n"
            "        if operation_code is not None:\n"
            "            kwargs['operation_code'] = int(operation_code)\n"
            "        if prefer_official_operation is not None:\n"
            "            kwargs['prefer_official_operation'] = prefer_official_operation\n"
            "        if epoch_policy is not None and str(epoch_policy).strip():\n"
            "            kwargs['epoch_policy'] = str(epoch_policy)\n"
            "        out_obj = wbe.reproject_raster(src, **kwargs)\n"
            "        wbe.write_raster(out_obj, out_path)\n"
            "    elif tool_id == 'reproject_vector':\n"
            "        src = wbe.read_vector(in_path)\n"
            "        kwargs = {'dst_epsg': epsg}\n"
            "        if coordinate_epoch is not None:\n"
            "            kwargs['coordinate_epoch'] = float(coordinate_epoch)\n"
            "        if source_reference_epoch is not None:\n"
            "            kwargs['source_reference_epoch'] = float(source_reference_epoch)\n"
            "        if target_reference_epoch is not None:\n"
            "            kwargs['target_reference_epoch'] = float(target_reference_epoch)\n"
            "        if operation_code is not None:\n"
            "            kwargs['operation_code'] = int(operation_code)\n"
            "        if prefer_official_operation is not None:\n"
            "            kwargs['prefer_official_operation'] = prefer_official_operation\n"
            "        if epoch_policy is not None and str(epoch_policy).strip():\n"
            "            kwargs['epoch_policy'] = str(epoch_policy)\n"
            "        out_obj = wbe.reproject_vector(src, **kwargs)\n"
            "        wbe.write_vector(out_obj, out_path)\n"
            "    elif tool_id == 'reproject_lidar':\n"
            "        src = wbe.read_lidar(in_path)\n"
            "        kwargs = {'dst_epsg': epsg}\n"
            "        if coordinate_epoch is not None:\n"
            "            kwargs['coordinate_epoch'] = float(coordinate_epoch)\n"
            "        if source_reference_epoch is not None:\n"
            "            kwargs['source_reference_epoch'] = float(source_reference_epoch)\n"
            "        if target_reference_epoch is not None:\n"
            "            kwargs['target_reference_epoch'] = float(target_reference_epoch)\n"
            "        if operation_code is not None:\n"
            "            kwargs['operation_code'] = int(operation_code)\n"
            "        if prefer_official_operation is not None:\n"
            "            kwargs['prefer_official_operation'] = prefer_official_operation\n"
            "        if epoch_policy is not None and str(epoch_policy).strip():\n"
            "            kwargs['epoch_policy'] = str(epoch_policy)\n"
            "        out_obj = wbe.reproject_lidar(src, **kwargs)\n"
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
    if os.name != "nt" or not raw:
        return None

    try:
        exe_path = Path(raw)
    except Exception:
        return None

    if not exe_path.exists():
        return None

    # Typical QGIS launcher layout:
    # C:\Program Files\QGIS <ver>\bin\qgis-bin.exe -> ..\apps\Python<xy>\python.exe
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
    """
    Discover all available external Python interpreters across Windows, macOS, and Linux.
    Returns candidates in priority order, searching both PATH and common installation locations.
    """
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

    # Environment-specified Python takes highest priority
    env_candidate = os.environ.get("WBW_EXTERNAL_PYTHON")
    _add_candidate(env_candidate)

    # Development venvs (user's local development environments)
    default_candidates = [
        Path.home() / "Documents" / "programming" / "Rust" / "whitebox_next_gen" / ".venv-wbw" / "bin" / "python",
        Path.home() / "Documents" / "programming" / "python" / ".venv" / "bin" / "python",
        Path.home() / ".venv" / "bin" / "python",
    ]

    # Platform-specific system and homebrew installations
    if os.name == "nt":  # Windows
        # Search PATH for python3 and python
        for py_name in ["python3", "python"]:
            resolved = shutil.which(py_name)
            _add_candidate(resolved)
        
        # Official python.org installer (C:\Python3X\python.exe pattern)
        try:
            import glob as glob_module
            for py_exe in glob_module.glob(r"C:\Python3*\python.exe"):
                _add_candidate(py_exe)
        except Exception:
            pass
        
        # Anaconda/Miniconda - user installation
        user_anaconda_base = Path.home() / "Anaconda3"
        if not user_anaconda_base.exists():
            user_anaconda_base = Path.home() / "Miniconda3"
        if user_anaconda_base.exists():
            _add_candidate(str(user_anaconda_base / "python.exe"))
        
        # Anaconda/Miniconda - system-wide installation
        system_anaconda_base = Path("C:\\ProgramData\\Anaconda3")
        if not system_anaconda_base.exists():
            system_anaconda_base = Path("C:\\ProgramData\\Miniconda3")
        if system_anaconda_base.exists():
            _add_candidate(str(system_anaconda_base / "python.exe"))
        
        # Microsoft Store installation
        appdata_python = Path.home() / "AppData" / "Local" / "Microsoft" / "WindowsApps" / "python3.exe"
        _add_candidate(str(appdata_python))

    else:  # macOS and Linux
        # Search PATH first (catches most installations)
        for py_name in ["python3", "python"]:
            resolved = shutil.which(py_name)
            _add_candidate(resolved)
        
        # macOS-specific paths
        if sys.platform == "darwin":
            macos_candidates = [
                Path("/opt/homebrew/bin/python3"),
                Path("/opt/homebrew/bin/python"),
                Path("/usr/local/bin/python3"),
                Path("/usr/local/bin/python"),
                Path("/usr/bin/python3"),
            ]
            for candidate in macos_candidates:
                _add_candidate(str(candidate))
        
        # Linux-specific paths
        else:
            linux_candidates = [
                Path("/usr/local/bin/python3"),
                Path("/usr/bin/python3"),
                Path("/usr/bin/python"),
            ]
            for candidate in linux_candidates:
                _add_candidate(str(candidate))

    # Add development venvs after system paths checked
    for candidate in default_candidates:
        _add_candidate(str(candidate))

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

            current = str(sys.executable)
            rewritten = _rewrite_windows_qgis_launcher(current)
            if _is_probably_python_executable(rewritten):
                return rewritten

            embedded = _discover_embedded_qgis_python(current)
            if _is_probably_python_executable(embedded):
                return embedded

            if _is_probably_python_executable(current):
                return current

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


def _get_python_version_tag() -> tuple[str, str]:
    """
    Return (major.minor, platform_tag) for current interpreter.
    E.g., ("3.10", "win_amd64") or ("3.11", "macosx_11_0_arm64")
    """
    major_minor = f"{sys.version_info.major}.{sys.version_info.minor}"
    
    if sys.platform == "win32":
        import struct
        platform_tag = "win_amd64" if struct.calcsize("P") == 8 else "win32"
    elif sys.platform == "darwin":
        import platform as platform_module
        machine = platform_module.machine()
        if machine == "arm64":
            platform_tag = "macosx_11_0_arm64"
        else:
            platform_tag = "macosx_10_9_x86_64"
    else:  # Linux
        platform_tag = "manylinux2014_x86_64" if sys.maxsize > 2**32 else "manylinux2014_i686"
    
    return major_minor, platform_tag


def _find_wheel_in_pypi_release(release_info: dict, py_version: str, platform_tag: str) -> dict | None:
    """
    Find the best matching wheel file from a PyPI release for the given Python version and platform.
    release_info is a list of file dicts from the PyPI JSON API.
    Returns the file dict or None if no match found.
    
    Supports both version-specific wheels (cp311) and stable ABI wheels (cp39-abi3).
    """
    if not isinstance(release_info, list):
        return None
    
    py_minor = int(py_version.split(".")[1])
    best_match = None
    best_score = -1  # Prefer specific version > abi3
    
    for file_info in release_info:
        filename = file_info.get("filename", "")
        if not filename.endswith(".whl"):
            continue
        
        # Parse wheel filename: {distribution}-{version}(-{build tag})?-{python tag}-{abi tag}-{platform tag}.whl
        parts = filename[:-4].split("-")  # Remove .whl and split
        if len(parts) < 5:
            continue
        
        py_tag = parts[-3]  # Python tag (e.g., "cp311", "cp39")
        abi_tag = parts[-2]  # ABI tag (e.g., "cp311", "abi3")
        plat_tag = parts[-1]  # Platform tag (e.g., "win_amd64", "manylinux2014_x86_64")
        
        # Match platform tag (normalize variations)
        platform_matches = False
        if "manylinux" in plat_tag and "manylinux" in platform_tag:
            # All manylinux versions are compatible
            platform_matches = True
        elif plat_tag == platform_tag:
            platform_matches = True
        elif "macosx" in plat_tag and "macosx" in platform_tag:
            # macOS wheel compatibility is more flexible (older wheels work on newer)
            platform_matches = True
        
        if not platform_matches:
            continue
        
        # Check Python version compatibility
        version_matches = False
        score = -1
        
        # Exact version match (highest priority)
        if py_tag == f"cp3{py_minor}":
            version_matches = True
            score = 10
        
        # Stable ABI (cp39-abi3) works on Python 3.9+
        elif py_tag == "cp39" and abi_tag == "abi3":
            if py_minor >= 9:
                version_matches = True
                score = 5
        
        # Any abi3 wheel where the minimum version is <= current version
        elif abi_tag == "abi3":
            # Extract minimum Python version from py_tag (e.g., "cp39" -> 9)
            try:
                min_py_minor = int(py_tag[2:])
                if py_minor >= min_py_minor:
                    version_matches = True
                    score = 5
            except (ValueError, IndexError):
                pass
        
        if version_matches and score > best_score:
            best_match = file_info
            best_score = score
    
    return best_match


def _get_os_wheel_platform() -> str:
    """
    Return the OS identifier for whiteboxgeo.com wheel downloads.
    Returns: "macos", "ubuntu", or "windows"
    """
    if sys.platform == "win32":
        return "windows"
    elif sys.platform == "darwin":
        return "macos"
    else:  # Linux
        return "ubuntu"


def _download_and_extract_wheel_from_whiteboxgeo(target_dir: str) -> bool:
    """
    Download whitebox-workflows wheel from whiteboxgeo.com and extract to target_dir.
    
    Strategy:
    1. Determine OS (macos, ubuntu, windows)
    2. Download the OS-specific ZIP file from whiteboxgeo.com
    3. Extract the wheel from the ZIP
    4. Find and extract the matching wheel to target_dir
    """
    try:
        os_platform = _get_os_wheel_platform()
        py_version, _ = _get_python_version_tag()
        
        # Construct download URL for OS-specific ZIP
        zip_url = f"https://www.whiteboxgeo.com/wbw_wheels/wbw-python-pro-{os_platform}-latest.zip"
        
        # Create SSL context
        ssl_context = ssl.create_default_context()
        ssl_context.check_hostname = False
        ssl_context.verify_mode = ssl.CERT_NONE
        
        # Download ZIP file
        with tempfile.TemporaryDirectory() as temp_dir:
            zip_path = os.path.join(temp_dir, f"wbw-wheels-{os_platform}.zip")
            urllib.request.urlretrieve(zip_url, zip_path)
            
            if not os.path.exists(zip_path):
                raise RuntimeBootstrapError(f"Failed to download wheel package from {zip_url}")
            
            # Extract ZIP to temp directory
            with zipfile.ZipFile(zip_path, "r") as zip_ref:
                zip_ref.extractall(temp_dir)
            
            # Find all wheel files in the extracted contents
            wheel_files = []
            for root, dirs, files in os.walk(temp_dir):
                for file in files:
                    if file.endswith(".whl"):
                        wheel_files.append(os.path.join(root, file))
            
            if not wheel_files:
                raise RuntimeBootstrapError(
                    f"No wheel files found in downloaded package from {zip_url}"
                )
            
            # Find the best matching wheel for current Python version
            # Try to find an exact or abi3 match
            best_wheel = None
            py_minor = int(py_version.split(".")[1])
            
            for wheel_path in wheel_files:
                wheel_filename = os.path.basename(wheel_path)
                
                # Parse wheel filename: {dist}-{version}-{python tag}-{abi tag}-{platform tag}.whl
                parts = wheel_filename[:-4].split("-")
                if len(parts) < 5:
                    continue
                
                py_tag = parts[-3]
                abi_tag = parts[-2]
                
                # Check for exact version match or abi3 wheel
                if py_tag == f"cp3{py_minor}":
                    best_wheel = wheel_path
                    break
                elif py_tag == "cp39" and abi_tag == "abi3" and not best_wheel:
                    # Keep abi3 wheel as fallback
                    best_wheel = wheel_path
                elif abi_tag == "abi3":
                    # Any abi3 wheel where min version <= current version
                    try:
                        min_py_minor = int(py_tag[2:])
                        if py_minor >= min_py_minor and not best_wheel:
                            best_wheel = wheel_path
                    except (ValueError, IndexError):
                        pass
            
            if not best_wheel:
                available = [os.path.basename(f) for f in wheel_files]
                raise RuntimeBootstrapError(
                    f"No compatible wheel found for Python {py_version}. "
                    f"Available: {available}"
                )
            
            # Extract the selected wheel to target_dir
            with zipfile.ZipFile(best_wheel, "r") as wheel_ref:
                wheel_ref.extractall(target_dir)
        
        return True
    
    except RuntimeBootstrapError:
        raise
    except Exception as exc:
        raise RuntimeBootstrapError(
            f"Failed to download/extract whitebox-workflows from whiteboxgeo.com: {str(exc)}"
        ) from exc


def _is_qgis_bundled_python(interpreter: str) -> bool:
    """
    Detect if the given interpreter is QGIS's bundled (sandboxed) Python.
    Heuristic: if running via subprocess and import fails but module is available
    in current process, likely QGIS bundled Python.
    """
    try:
        # If interpreter path contains 'qgis' (case-insensitive), likely QGIS Python
        if "qgis" in interpreter.lower():
            return True
        
        # Test if this Python can run pip
        result = subprocess.run(
            [interpreter, "-m", "pip", "--version"],
            capture_output=True,
            timeout=2,
            **_subprocess_window_kwargs(),
        )
        
        # If pip command fails, likely sandboxed
        return result.returncode != 0
    except Exception:
        return False


def install_or_upgrade_whitebox_workflows(*, upgrade: bool = False, version_spec: str = "") -> dict:
    """
    Install or upgrade whitebox-workflows using the best available strategy:
    1. If pip is available, use pip install
    2. If pip is not available (e.g., QGIS bundled Python), download wheel from PyPI and extract
    """
    interpreter = get_backend_interpreter_path()
    
    # Determine version to install
    target_version = str(version_spec or "").strip()
    if not target_version:
        try:
            target_version = fetch_latest_whitebox_workflows_version()
        except Exception:
            target_version = "latest"
    
    # Strategy 1: Try pip first (works for external Python)
    if not _is_qgis_bundled_python(interpreter):
        package_spec = "whitebox-workflows"
        if target_version and target_version != "latest":
            package_spec = f"whitebox-workflows{target_version}"
        
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
            **_subprocess_window_kwargs(),
        )
        stdout = completed.stdout.strip()
        stderr = completed.stderr.strip()
        
        if completed.returncode == 0:
            installed = ""
            try:
                installed = get_installed_whitebox_workflows_version()
            except Exception:
                installed = ""
            
            _EXTERNAL_SESSION_CACHE.clear()
            
            return {
                "interpreter": interpreter,
                "installed_version": installed,
                "stdout": stdout,
                "stderr": stderr,
                "upgraded": bool(upgrade),
                "strategy": "pip",
            }
    
    # Strategy 2: Use wheel download/extract from whiteboxgeo.com for QGIS bundled Python
    try:
        # Get plugin directory to extract wheel
        plugin_dir = os.path.dirname(os.path.realpath(__file__))
        
        _download_and_extract_wheel_from_whiteboxgeo(plugin_dir)
        
        installed = ""
        try:
            installed = get_installed_whitebox_workflows_version()
        except Exception:
            installed = ""
        
        _EXTERNAL_SESSION_CACHE.clear()
        
        return {
            "interpreter": interpreter,
            "installed_version": installed,
            "upgraded": bool(upgrade),
            "strategy": "wheel_whiteboxgeo",
        }
    
    except RuntimeBootstrapError as exc:
        # Strategy 3 (Fallback): If whiteboxgeo.com fails, try external Python with pip
        try:
            external_candidates = _discover_external_python_candidates()
            for external_py in external_candidates:
                try:
                    # Test if this Python has pip
                    test = subprocess.run(
                        [external_py, "-m", "pip", "--version"],
                        capture_output=True,
                        timeout=2,
                        **_subprocess_window_kwargs(),
                    )
                    if test.returncode != 0:
                        continue
                    
                    # Found a working pip, try to install
                    package_spec = "whitebox-workflows"
                    command = [external_py, "-m", "pip", "install"]
                    if upgrade:
                        command.append("--upgrade")
                    command.append(package_spec)
                    
                    completed = subprocess.run(
                        command,
                        check=False,
                        capture_output=True,
                        text=True,
                        env=_build_clean_env(),
                        **_subprocess_window_kwargs(),
                    )
                    
                    if completed.returncode == 0:
                        installed = ""
                        try:
                            installed = _get_installed_whitebox_workflows_version_for_interpreter(external_py)
                        except Exception:
                            installed = ""
                        
                        _EXTERNAL_SESSION_CACHE.clear()
                        
                        return {
                            "interpreter": external_py,
                            "installed_version": installed,
                            "upgraded": bool(upgrade),
                            "strategy": "pip_fallback",
                            "note": f"Installed via external Python: {external_py}",
                        }
                except Exception:
                    continue
        except Exception:
            pass
        
        # All strategies failed
        raise exc
    except Exception as exc:
        raise RuntimeBootstrapError(
            f"Failed to install whitebox-workflows from whiteboxgeo.com: {str(exc)}"
        ) from exc


def fetch_latest_whitebox_workflows_version(timeout_seconds: float = 4.0) -> str:
    url = "https://pypi.org/pypi/whitebox-workflows/json"
    parsed_url = urllib.parse.urlparse(url)
    if parsed_url.scheme != "https" or parsed_url.netloc != "pypi.org":
        raise RuntimeBootstrapError(
            "Refusing to query package index from non-HTTPS or untrusted host."
        )

    try:
        # Create SSL context to handle certificate verification
        ssl_context = ssl.create_default_context()
        # Disable SSL verification if needed (for corporate networks, etc.)
        ssl_context.check_hostname = False
        ssl_context.verify_mode = ssl.CERT_NONE
        
        # URL is validated above (scheme + host allowlist).
        with urllib.request.urlopen(url, timeout=float(timeout_seconds), context=ssl_context) as response:  # nosec B310
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

    def _to_optional_bool(v):
        if v is None:
            return None
        if isinstance(v, bool):
            return v
        if isinstance(v, str):
            t = v.strip().lower()
            if t in {"true", "1", "yes", "y", "on"}:
                return True
            if t in {"false", "0", "no", "n", "off"}:
                return False
            return None
        return bool(v)

    epsg = int(args.get("epsg"))
    in_path = str(args.get("input", ""))
    out_path = str(args.get("output", ""))
    coordinate_epoch = args.get("coordinate_epoch", None)
    source_reference_epoch = args.get("source_reference_epoch", None)
    target_reference_epoch = args.get("target_reference_epoch", None)
    operation_code = args.get("operation_code", None)
    prefer_official_operation = _to_optional_bool(args.get("prefer_official_operation", None))
    epoch_policy = args.get("epoch_policy", None)

    if tool_id == "reproject_raster":
        resample = str(args.get("resample", "bilinear")).strip() or "bilinear"
        src = wbe.read_raster(in_path)
        kwargs = {"dst_epsg": epsg, "resample": resample}
        if coordinate_epoch is not None:
            kwargs["coordinate_epoch"] = float(coordinate_epoch)
        if source_reference_epoch is not None:
            kwargs["source_reference_epoch"] = float(source_reference_epoch)
        if target_reference_epoch is not None:
            kwargs["target_reference_epoch"] = float(target_reference_epoch)
        if operation_code is not None:
            kwargs["operation_code"] = int(operation_code)
        if prefer_official_operation is not None:
            kwargs["prefer_official_operation"] = prefer_official_operation
        if epoch_policy is not None and str(epoch_policy).strip():
            kwargs["epoch_policy"] = str(epoch_policy)
        out_obj = wbe.reproject_raster(src, **kwargs)
        wbe.write_raster(out_obj, out_path)
    elif tool_id == "reproject_vector":
        src = wbe.read_vector(in_path)
        kwargs = {"dst_epsg": epsg}
        if coordinate_epoch is not None:
            kwargs["coordinate_epoch"] = float(coordinate_epoch)
        if source_reference_epoch is not None:
            kwargs["source_reference_epoch"] = float(source_reference_epoch)
        if target_reference_epoch is not None:
            kwargs["target_reference_epoch"] = float(target_reference_epoch)
        if operation_code is not None:
            kwargs["operation_code"] = int(operation_code)
        if prefer_official_operation is not None:
            kwargs["prefer_official_operation"] = prefer_official_operation
        if epoch_policy is not None and str(epoch_policy).strip():
            kwargs["epoch_policy"] = str(epoch_policy)
        out_obj = wbe.reproject_vector(src, **kwargs)
        wbe.write_vector(out_obj, out_path)
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
        kwargs = {"dst_epsg": epsg}
        if coordinate_epoch is not None:
            kwargs["coordinate_epoch"] = float(coordinate_epoch)
        if source_reference_epoch is not None:
            kwargs["source_reference_epoch"] = float(source_reference_epoch)
        if target_reference_epoch is not None:
            kwargs["target_reference_epoch"] = float(target_reference_epoch)
        if operation_code is not None:
            kwargs["operation_code"] = int(operation_code)
        if prefer_official_operation is not None:
            kwargs["prefer_official_operation"] = prefer_official_operation
        if epoch_policy is not None and str(epoch_policy).strip():
            kwargs["epoch_policy"] = str(epoch_policy)
        out_obj = wbe.reproject_lidar(src, **kwargs)
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
