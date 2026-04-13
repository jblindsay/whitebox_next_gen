from .whitebox_workflows import *
from . import callbacks
from .callbacks import ProgressPrinter, make_progress_printer, print_progress


def _add_if_not_none(args, key, value):
    if value is not None:
        args[key] = value


def analyze_multimodal_od_scenarios(
    env,
    input,
    origins,
    destinations,
    output,
    mode_field="MODE",
    allowed_modes=None,
    mode_speed_overrides=None,
    transfer_penalty=None,
    edge_cost_field=None,
    max_snap_distance=None,
    scenario_bundle_csv=None,
    temporal_cost_profile=None,
    departure_time=None,
    temporal_mode=None,
    parallel_execution=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
        "mode_field": mode_field,
    }
    _add_if_not_none(args, "allowed_modes", allowed_modes)
    _add_if_not_none(args, "mode_speed_overrides", mode_speed_overrides)
    _add_if_not_none(args, "transfer_penalty", transfer_penalty)
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "scenario_bundle_csv", scenario_bundle_csv)
    _add_if_not_none(args, "temporal_cost_profile", temporal_cost_profile)
    _add_if_not_none(args, "departure_time", departure_time)
    _add_if_not_none(args, "temporal_mode", temporal_mode)
    _add_if_not_none(args, "parallel_execution", parallel_execution)
    return env.run_tool("multimodal_od_cost_matrix", args)


def export_multimodal_routes_for_od_pairs(
    env,
    input,
    origins,
    destinations,
    output,
    mode_field="MODE",
    allowed_modes=None,
    mode_speed_overrides=None,
    transfer_penalty=None,
    edge_cost_field=None,
    max_snap_distance=None,
    scenario_bundle_csv=None,
    temporal_cost_profile=None,
    departure_time=None,
    temporal_mode=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
        "mode_field": mode_field,
    }
    _add_if_not_none(args, "allowed_modes", allowed_modes)
    _add_if_not_none(args, "mode_speed_overrides", mode_speed_overrides)
    _add_if_not_none(args, "transfer_penalty", transfer_penalty)
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "scenario_bundle_csv", scenario_bundle_csv)
    _add_if_not_none(args, "temporal_cost_profile", temporal_cost_profile)
    _add_if_not_none(args, "departure_time", departure_time)
    _add_if_not_none(args, "temporal_mode", temporal_mode)
    return env.run_tool("multimodal_routes_from_od", args)


def compute_network_accessibility(
    env,
    input,
    origins,
    destinations,
    output,
    edge_cost_field=None,
    max_snap_distance=None,
    impedance_cutoff=None,
    decay_function=None,
    decay_parameter=None,
    parallel_execution=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
    }
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "impedance_cutoff", impedance_cutoff)
    _add_if_not_none(args, "decay_function", decay_function)
    _add_if_not_none(args, "decay_parameter", decay_parameter)
    _add_if_not_none(args, "parallel_execution", parallel_execution)
    return env.run_tool("network_accessibility_metrics", args)


def analyze_od_cost_sensitivity(
    env,
    input,
    origins,
    destinations,
    output,
    edge_cost_field=None,
    max_snap_distance=None,
    impedance_disturbance_range=None,
    monte_carlo_samples=None,
    parallel_execution=None,
):
    args = {
        "input": input,
        "origins": origins,
        "destinations": destinations,
        "output": output,
    }
    _add_if_not_none(args, "edge_cost_field", edge_cost_field)
    _add_if_not_none(args, "max_snap_distance", max_snap_distance)
    _add_if_not_none(args, "impedance_disturbance_range", impedance_disturbance_range)
    _add_if_not_none(args, "monte_carlo_samples", monte_carlo_samples)
    _add_if_not_none(args, "parallel_execution", parallel_execution)
    return env.run_tool("od_sensitivity_analysis", args)


def _attach_phase4_convenience_methods():
    if "WbEnvironment" not in globals():
        return
    WbEnvironment.analyze_multimodal_od_scenarios = analyze_multimodal_od_scenarios
    WbEnvironment.export_multimodal_routes_for_od_pairs = export_multimodal_routes_for_od_pairs
    WbEnvironment.compute_network_accessibility = compute_network_accessibility
    WbEnvironment.analyze_od_cost_sensitivity = analyze_od_cost_sensitivity


_attach_phase4_convenience_methods()

__doc__ = whitebox_workflows.__doc__
if hasattr(whitebox_workflows, "__all__"):
    __all__ = list(whitebox_workflows.__all__) + [
        "callbacks",
        "ProgressPrinter",
        "make_progress_printer",
        "print_progress",
        "analyze_multimodal_od_scenarios",
        "export_multimodal_routes_for_od_pairs",
        "compute_network_accessibility",
        "analyze_od_cost_sensitivity",
    ]
else:
    __all__ = [
        "callbacks",
        "ProgressPrinter",
        "make_progress_printer",
        "print_progress",
        "analyze_multimodal_od_scenarios",
        "export_multimodal_routes_for_od_pairs",
        "compute_network_accessibility",
        "analyze_od_cost_sensitivity",
    ]
