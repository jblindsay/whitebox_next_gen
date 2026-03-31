#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: bash scripts/set_backend_repo_metadata.sh <repo_url>"
  echo "Example: bash scripts/set_backend_repo_metadata.sh https://github.com/ORG/whitebox_next_gen"
  exit 1
fi

repo_url="$1"

if [[ ! "$repo_url" =~ ^https://github.com/[^/]+/[^/]+$ ]]; then
  echo "Error: repo_url must look like https://github.com/ORG/REPO"
  exit 1
fi

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

for crate in "${backend_crates[@]}"; do
  manifest="crates/$crate/Cargo.toml"
  echo "Updating $manifest"

  # Normalize key metadata lines if they already exist.
  if grep -Eq '^[[:space:]]*repository[[:space:]]*=' "$manifest"; then
    sed -i '' -E "s|^[[:space:]]*repository[[:space:]]*=.*$|repository = \"$repo_url\"|" "$manifest"
  fi
  if grep -Eq '^[[:space:]]*homepage[[:space:]]*=' "$manifest"; then
    sed -i '' -E "s|^[[:space:]]*homepage[[:space:]]*=.*$|homepage = \"$repo_url\"|" "$manifest"
  fi
  if grep -Eq '^[[:space:]]*documentation[[:space:]]*=' "$manifest"; then
    sed -i '' -E "s|^[[:space:]]*documentation[[:space:]]*=.*$|documentation = \"https://docs.rs/$crate\"|" "$manifest"
  fi

  # Insert missing lines just after [package] metadata header block.
  if ! grep -Eq '^[[:space:]]*repository[[:space:]]*=' "$manifest"; then
    sed -i '' -E "/^[[:space:]]*readme[[:space:]]*=.*/a\\
repository = \"$repo_url\"" "$manifest"
  fi

  if ! grep -Eq '^[[:space:]]*homepage[[:space:]]*=' "$manifest"; then
    sed -i '' -E "/^[[:space:]]*repository[[:space:]]*=.*/a\\
homepage = \"$repo_url\"" "$manifest"
  fi

  if ! grep -Eq '^[[:space:]]*documentation[[:space:]]*=' "$manifest"; then
    sed -i '' -E "/^[[:space:]]*homepage[[:space:]]*=.*/a\\
documentation = \"https://docs.rs/$crate\"" "$manifest"
  fi
done

echo "Done."
