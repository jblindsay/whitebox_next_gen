import argparse
import json
from pathlib import Path

import whitebox_workflows


def print_event(event_json: str) -> None:
    """Print progress and message events from a tool."""
    event = json.loads(event_json)
    etype = event.get("type")
    if etype == "message":
        print(f"[message] {event.get('message', '')}")
    elif etype == "progress":
        pct = float(event.get("percent", 0.0)) * 100.0
        print(f"[progress] {pct:6.2f}%")


def run_unary_tool_simple(
    tool_id: str,
    input_raster: whitebox_workflows.Raster,
    output_path: Path | None,
) -> dict:
    """Run a unary tool using the simple API: whitebox_workflows.tool_name(raster, output?)."""
    # Get the tool function from the module
    tool_func = getattr(whitebox_workflows, tool_id)
    result_json = tool_func(input_raster, None if output_path is None else str(output_path))
    return json.loads(result_json)


def run_unary_tool_with_callback(
    tool_id: str,
    input_raster: whitebox_workflows.Raster,
    output_path: Path | None,
) -> dict:
    """Run a unary tool with live progress callback."""
    tool_func = getattr(whitebox_workflows, tool_id)
    result_json = tool_func(
        input_raster,
        None if output_path is None else str(output_path),
        callback=print_event,
    )
    return json.loads(result_json)


def parse_tool_list(raw: str) -> list[str]:
    tools = [t.strip() for t in raw.split(",") if t.strip()]
    if not tools:
        raise ValueError("At least one tool id must be supplied")
    return tools


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Whitebox unary raster tools from whitebox_workflows."
    )
    parser.add_argument("input", type=Path, help="Input raster path")
    parser.add_argument("output_dir", type=Path, help="Directory where outputs are written")
    parser.add_argument(
        "--tools",
        default="abs,sqrt",
        help="Comma-separated unary tool ids to run. Example: abs,ceil,floor,round,sqrt,square,ln,log10,sin,cos",
    )
    parser.add_argument(
        "--progress",
        action="store_true",
        help="Show live progress updates",
    )
    parser.add_argument(
        "--auto-output",
        action="store_true",
        help="Let the API auto-generate output file names",
    )
    args = parser.parse_args()

    if not args.input.exists():
        raise FileNotFoundError(f"Input raster does not exist: {args.input}")

    tool_ids = parse_tool_list(args.tools)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    input_raster = whitebox_workflows.Raster(str(args.input))

    for tool_id in tool_ids:
        out_path = args.output_dir / f"{args.input.stem}_{tool_id}{args.input.suffix}"
        print(f"\nRunning {tool_id}:\n  in:  {args.input}\n  out: {out_path}")
        requested_output = None if args.auto_output else out_path

        if args.progress:
            result = run_unary_tool_with_callback(tool_id, input_raster, requested_output)
        else:
            result = run_unary_tool_simple(tool_id, input_raster, requested_output)

        outputs = result.get("outputs", result)
        print(f"Completed {tool_id}; output: {outputs.get('output', out_path)}")


if __name__ == "__main__":
    main()

