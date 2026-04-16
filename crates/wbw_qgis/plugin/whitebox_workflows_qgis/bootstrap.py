from __future__ import annotations

import base64
import importlib
import json
import os
import shutil
import subprocess
from pathlib import Path


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
    return (
        "include_pro=true requested" in text
        and "does not include pro support" in text
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
            "        src = wbe.read_raster(in_path)\n"
            "        out_obj = wbe.reproject_raster(src, dst_epsg=epsg)\n"
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

        clean_env = dict(os.environ)
        clean_env.pop("PYTHONHOME", None)
        clean_env.pop("PYTHONPATH", None)

        completed = subprocess.run(
            [self._python_executable, "-c", runner, json.dumps(payload)],
            check=False,
            capture_output=True,
            text=True,
            env=clean_env,
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
            "if isinstance(out, str):\n"
            "    out_text = out\n"
            "else:\n"
            "    out_text = json.dumps(out)\n"
            "enc_out = base64.b64encode(out_text.encode('utf-8')).decode('ascii')\n"
            "sys.stdout.write('__WBW_RESULT__' + enc_out + '\\n')\n"
            "sys.stdout.flush()\n"
        )

        clean_env = dict(os.environ)
        clean_env.pop("PYTHONHOME", None)
        clean_env.pop("PYTHONPATH", None)

        completed_result: str | None = None
        loose_stdout_lines: list[str] = []

        process = subprocess.Popen(
            [self._python_executable, "-c", runner, json.dumps(payload)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=clean_env,
            bufsize=1,
        )

        assert process.stdout is not None
        for raw_line in process.stdout:
            line = raw_line.rstrip("\r\n")
            if line.startswith("__WBW_EVENT__"):
                enc = line[len("__WBW_EVENT__") :]
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
                enc = line[len("__WBW_RESULT__") :]
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
    env_candidate = os.environ.get("WBW_EXTERNAL_PYTHON")
    if env_candidate:
        p = Path(env_candidate).expanduser()
        if p.exists() and os.access(p, os.X_OK):
            return str(p)

    default_candidates = [
        Path.home() / "Documents" / "programming" / "python" / ".venv" / "bin" / "python",
        Path.home() / ".venv" / "bin" / "python",
    ]
    for candidate in default_candidates:
        if candidate.exists() and os.access(candidate, os.X_OK):
            return str(candidate)

    resolved = shutil.which("python3")
    if resolved:
        return resolved
    return None


def load_whitebox_workflows():
    try:
        return importlib.import_module("whitebox_workflows")
    except ImportError as exc:
        raise RuntimeBootstrapError(
            "The whitebox_workflows package is not available in this Python environment. "
            "Install or activate a QGIS-compatible WbW-Py build before loading the plugin."
        ) from exc


def create_runtime_session(include_pro: bool = True, tier: str = "open"):
    def _external_session(prefer_pro: bool, allow_downgrade: bool = True):
        external_python = _discover_external_python()
        if not external_python:
            raise RuntimeBootstrapError(
                "No external Python interpreter was found for whitebox_workflows fallback."
            )

        session = ExternalRuntimeSession(external_python, include_pro=prefer_pro, tier=tier)
        try:
            raw_caps = session.get_runtime_capabilities_json()
            _parse_next_gen_capabilities(raw_caps, f"external runtime ({external_python})")
            return session
        except RuntimeBootstrapError as exc:
            if allow_downgrade and prefer_pro and _is_pro_unavailable_error(str(exc)):
                downgraded = ExternalRuntimeSession(external_python, include_pro=False, tier=tier)
                raw_caps = downgraded.get_runtime_capabilities_json()
                _parse_next_gen_capabilities(raw_caps, f"external runtime ({external_python})")
                return downgraded
            raise

    try:
        wbw = load_whitebox_workflows()
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
                # Prefer an external runtime that may include Pro manifests before
                # downgrading to OSS-only behavior.
                try:
                    return _external_session(prefer_pro=True, allow_downgrade=True)
                except Exception:
                    downgraded = wbw.RuntimeSession(include_pro=False, tier=tier)
                    raw_caps = downgraded.get_runtime_capabilities_json()
                    _parse_next_gen_capabilities(raw_caps, "current Python runtime")
                    return downgraded
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
        src = wbe.read_raster(in_path)
        out_obj = wbe.reproject_raster(src, dst_epsg=epsg)
        wbe.write_raster(out_obj, out_path)
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