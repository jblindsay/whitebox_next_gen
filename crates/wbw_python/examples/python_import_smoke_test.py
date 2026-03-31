import json
import pathlib
import shutil
import sys
import sysconfig
import tempfile


def main() -> None:
    root = pathlib.Path(__file__).resolve().parents[3]
    lib = root / "target" / "debug" / "libwhitebox_workflows.dylib"
    if not lib.exists():
        raise RuntimeError(f"missing built library: {lib}")

    suffix = sysconfig.get_config_var("EXT_SUFFIX")
    if not suffix:
        raise RuntimeError("missing EXT_SUFFIX")

    with tempfile.TemporaryDirectory() as td:
        td_path = pathlib.Path(td)
        mod_path = td_path / f"whitebox_workflows{suffix}"
        shutil.copy2(lib, mod_path)
        sys.path.insert(0, str(td_path))

        import whitebox_workflows

        tools = json.loads(whitebox_workflows.list_tools_json())
        ids = {t["id"] for t in tools}
        assert "raster_add_constant" in ids

        out = json.loads(
            whitebox_workflows.run_tool_json(
                "raster_add_constant", '{"input":[1,2],"constant":4}'
            )
        )
        assert out["result"] == [5.0, 6.0]

        session = whitebox_workflows.RuntimeSession()
        out2 = json.loads(
            session.run_tool_json(
                "raster_multiply_constant", '{"input":[3],"constant":2}'
            )
        )
        assert out2["result"] == [6.0]

    print("python import smoke test passed")


if __name__ == "__main__":
    main()
