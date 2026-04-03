#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

continue_on_error=0
if [[ "${1:-}" == "--continue" ]]; then
  continue_on_error=1
fi

publish_order=(
  wbgeotiff
  wbprojection
  wbraster
  wbvector
  wbtopology
  wblidar
)

non_publishable_crates=(
  wbcore
  wblicense_core
  wbtools_oss
  wbphotogrammetry
  wbw_python
  wbw_r
)

echo "Repository: $repo_root"
echo "Dry-run mode: cargo package --allow-dirty --no-verify"
echo "Publish order: ${publish_order[*]}"
echo "Excluded from crates.io publishing: ${non_publishable_crates[*]}"
echo ""

failed=0
for crate in "${publish_order[@]}"; do
  echo "== $crate =="
  if output="$(cargo package -p "$crate" --allow-dirty --no-verify 2>&1)"; then
    echo "PASS"
  else
    failed=1
    echo "FAIL"
    echo "$output" | sed 's/^/  /'

    if echo "$output" | grep -q "no matching package named"; then
      missing="$(echo "$output" | sed -n 's/^  no matching package named `\([^`]*\)` found$/\1/p' | head -1)"
      if [[ -n "$missing" ]]; then
        echo ""
        echo "Blocked by publish order: dependency '$missing' is not yet available on crates.io."
      fi
    fi

    if [[ "$continue_on_error" -eq 0 ]]; then
      echo ""
      echo "Stopping on first failure. Re-run with --continue to report all failures."
      exit 1
    fi
  fi
  echo ""
done

if [[ "$failed" -eq 0 ]]; then
  echo "All backend crates passed dry-run packaging checks in publish order."
  exit 0
fi

echo "One or more crates failed dry-run packaging checks."
exit 1
