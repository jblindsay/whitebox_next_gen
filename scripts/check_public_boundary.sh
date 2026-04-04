#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

# Sensitive paths that must not be pushed to public mainline by default.
restricted_paths=(
  "crates/wbcore/"
  "crates/wblicense_core/"
  "crates/wbtools_oss/"
  "crates/wbw_python/"
  "crates/wbw_r/"
)

# Explicit emergency override (must be intentional and auditable in CI logs).
if [[ "${PUBLIC_BOUNDARY_OVERRIDE:-}" == "I_UNDERSTAND_THIS_IS_PUBLIC" ]]; then
  echo "PUBLIC_BOUNDARY_OVERRIDE set; boundary guard bypassed intentionally."
  exit 0
fi

resolve_base_ref() {
  if [[ -n "${1:-}" ]]; then
    echo "$1"
    return
  fi

  if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
    echo "origin/${GITHUB_BASE_REF}"
    return
  fi

  if git rev-parse --verify origin/main >/dev/null 2>&1; then
    echo "origin/main"
    return
  fi

  if git rev-parse --verify main >/dev/null 2>&1; then
    echo "main"
    return
  fi

  if git rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    echo "HEAD~1"
    return
  fi

  echo ""
}

base_ref="$(resolve_base_ref "${1:-}")"

if [[ -z "$base_ref" ]]; then
  echo "No usable base ref found; skipping boundary check."
  exit 0
fi

if ! git rev-parse --verify "$base_ref" >/dev/null 2>&1; then
  echo "Base ref '$base_ref' is not available locally; trying to fetch origin/main."
  git fetch origin main --quiet || true
fi

if ! git rev-parse --verify "$base_ref" >/dev/null 2>&1; then
  echo "Base ref '$base_ref' still unavailable; skipping boundary check."
  exit 0
fi

changed_files="$(git diff --name-only "$base_ref"...HEAD)"
if [[ -z "$changed_files" ]]; then
  echo "No changed files detected relative to $base_ref."
  exit 0
fi

violations=()
while IFS= read -r file; do
  for restricted in "${restricted_paths[@]}"; do
    if [[ "$file" == "$restricted"* ]]; then
      violations+=("$file")
      break
    fi
  done
done <<< "$changed_files"

if (( ${#violations[@]} == 0 )); then
  echo "Public boundary guard passed."
  exit 0
fi

echo "Public boundary guard failed. Restricted paths changed:"
for v in "${violations[@]}"; do
  echo "  - $v"
done

echo ""
echo "If this is truly intentional, set:"
echo "  PUBLIC_BOUNDARY_OVERRIDE=I_UNDERSTAND_THIS_IS_PUBLIC"
echo "and rerun explicitly."

exit 1
