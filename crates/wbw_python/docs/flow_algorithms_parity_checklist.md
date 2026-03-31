# Flow Algorithms Parity Checklist

Reference legacy source:
- /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology

Scope:
- New backend implementations in wbtools_oss
- Focused on flow direction and flow accumulation algorithms

## Status Legend
- done: implemented and compiled in new backend
- partial: present but still behaviorally simplified vs legacy
- todo: not yet implemented in new backend

## Flow Direction Tools
- d8_pointer: done
- dinf_pointer: done
- fd8_pointer: done
- rho8_pointer: done

## Flow Accumulation Tools
- d8_flow_accum: done
- dinf_flow_accum: done
- fd8_flow_accum: done
- mdinf_flow_accum: done
- qin_flow_accumulation: done
- quinn_flow_accumulation: done
- rho8_flow_accum: done

## Combined Direction + Accumulation Tool
- minimal_dispersion_flow_algorithm: done
  - Note: This is a combined flow-direction and flow-accumulation method, not solely a pointer tool.

## Related Workflow
- flow_accum_full_workflow: done

## Validation Status
- cargo check -p wbtools_oss -p wbtools_pro: pass
- cargo test -p wbtools_oss --test registry_integration -- --nocapture: pass

## Next Porting Order
1. Validate strict parity details and expand behavioral tests
