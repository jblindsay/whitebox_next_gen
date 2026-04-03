#!/usr/bin/env bash
set -euo pipefail

# Run from anywhere inside the repository.
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

backend_crates=(
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

publish_order=(
  wbgeotiff
  wbprojection
  wbraster
  wbvector
  wbtopology
  wblidar
)

required_fields=(
  "name"
  "version"
  "description"
  "license"
  "readme"
  "repository"
)

# Files/paths that should never appear in published backend crate packages.
deny_pattern='(^|/)examples/internal/|(^|/)dev/|(^|/)docs/internal/|(^|/)tools/|(^|/)artifacts/|(^|/)IMPLEMENTATION_PLAN\.md$|(^|/)TOOL_INVENTORY|(^|/)PARALLELIZATION_SIMD_AUDIT\.md$|(^|/)LAS_V15_IMPLEMENTATION_PLAN\.md$'

echo "Repository: $repo_root"
echo ""
echo "Backend crates: ${backend_crates[*]}"
echo "Non-publishable crates: ${non_publishable_crates[*]}"
echo "Recommended publish order: ${publish_order[*]}"
echo ""

echo "== Metadata checks =="
metadata_fail=0
for crate in "${backend_crates[@]}"; do
  manifest="crates/$crate/Cargo.toml"
  echo "-- $crate"
  for field in "${required_fields[@]}"; do
    if grep -Eq "^[[:space:]]*$field[[:space:]]*=" "$manifest"; then
      echo "   ok: $field"
    else
      echo "   MISSING: $field"
      metadata_fail=1
    fi
  done
done

echo ""
echo "== Non-publishable crate policy =="
non_publishable_fail=0
for crate in "${non_publishable_crates[@]}"; do
  manifest="crates/$crate/Cargo.toml"
  echo "-- $crate"
  if grep -Eq '^[[:space:]]*publish[[:space:]]*=[[:space:]]*false[[:space:]]*$' "$manifest"; then
    echo "   ok: publish = false"
  else
    echo "   MISSING: publish = false"
    non_publishable_fail=1
  fi
done

echo ""
echo "== Strict package surface policy =="
policy_fail=0
for crate in "${backend_crates[@]}"; do
  echo "-- $crate"
  if package_list="$(cargo package -p "$crate" --allow-dirty --list 2>&1)"; then
    if violations="$(echo "$package_list" | grep -E "$deny_pattern" || true)"; then
      if [[ -n "$violations" ]]; then
        echo "   FAIL: internal/development files found in package list"
        echo "$violations" | sed 's/^/   /'
        policy_fail=1
      else
        echo "   PASS"
      fi
    fi
  else
    echo "   FAIL: could not list package files"
    echo "$package_list" | sed 's/^/   /'
    policy_fail=1
  fi
done

echo ""
echo "== Packaging checks (cargo package --allow-dirty --no-verify) =="
package_fail=0
for crate in "${backend_crates[@]}"; do
  echo "-- $crate"
  if output="$(cargo package -p "$crate" --allow-dirty --no-verify 2>&1)"; then
    echo "   PASS"
  else
    echo "   FAIL"
    echo "$output" | sed 's/^/   /'
    package_fail=1
  fi
done

echo ""
if [[ "$metadata_fail" -eq 0 && "$non_publishable_fail" -eq 0 && "$policy_fail" -eq 0 && "$package_fail" -eq 0 ]]; then
  echo "All checks passed."
  exit 0
fi

echo "One or more checks failed. See output above."
exit 1
