#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

usage() {
  cat <<'EOF'
Usage:
  bash scripts/run_maintainer_workflows.sh list
  bash scripts/run_maintainer_workflows.sh topology-perf-gate [args...]
  bash scripts/run_maintainer_workflows.sh projection-list-tools
  bash scripts/run_maintainer_workflows.sh topology-list-tools

Notes:
  - Maintainer-only internal files are intentionally under dev/ and excluded from published crates.
  - topology-perf-gate runs wbtopology perf regression gate from dev/scripts.
EOF
}

cmd="${1:-}"
if [[ -z "$cmd" ]]; then
  usage
  exit 1
fi

shift || true

case "$cmd" in
  list)
    usage
    ;;
  topology-perf-gate)
    bash crates/wbtopology/dev/scripts/overlay_perf_gate.sh "$@"
    ;;
  projection-list-tools)
    find crates/wbprojection/dev/examples/internal -maxdepth 1 -type f -name "*.rs" | sort
    ;;
  topology-list-tools)
    find crates/wbtopology/dev/examples/internal -maxdepth 1 -type f -name "*.rs" | sort
    ;;
  *)
    echo "Unknown command: $cmd"
    echo ""
    usage
    exit 1
    ;;
esac
