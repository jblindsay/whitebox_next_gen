#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

hook_path=".git/hooks/pre-push"
mkdir -p "$(dirname "$hook_path")"

cat > "$hook_path" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
"$repo_root/scripts/check_public_boundary.sh" origin/main
HOOK

chmod +x "$hook_path"

echo "Installed boundary pre-push hook at $hook_path"
echo "This blocks pushes that modify restricted public-sensitive paths."
