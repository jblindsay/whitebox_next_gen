#!/usr/bin/env bash
set -euo pipefail

# One-shot helper for local v2 entitlement issuance and client bootstrap snippet generation.
# Requirements:
# - Radiant Garden running with v2 endpoints enabled
# - WBW_ADMIN_SECRET set in your shell
# Optional env:
# - WBW_SERVER_URL (default: http://127.0.0.1:8080)
# - ALLOW_NONLOCAL (default: unset; set to 1 to allow non-local server URLs)
# - WBW_PRODUCT (default: whitebox_next_gen)
# - WBW_TIER (default: pro)
# - WBW_MAX_TIER (default: pro)
# - WBW_ISSUER (default: radiant-garden-local)
# - WBW_ALLOWED_TOOL_IDS (comma-separated, default: empty)
# - WBW_DAYS_VALID (default: 7)
# - WBW_CUSTOMER_ID (default: john-local)
# - WBW_OUT_DIR (default: ./target/license_test)

if ! command -v curl >/dev/null 2>&1; then
  echo "Error: curl is required." >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "Error: python3 is required." >&2
  exit 1
fi

if [[ -z "${WBW_ADMIN_SECRET:-}" ]]; then
  echo "Error: WBW_ADMIN_SECRET is not set." >&2
  echo "Set it first, for example:" >&2
  echo "  export WBW_ADMIN_SECRET='your-admin-secret'" >&2
  exit 1
fi

SERVER_URL="${WBW_SERVER_URL:-http://127.0.0.1:8080}"
PRODUCT="${WBW_PRODUCT:-whitebox_next_gen}"
TIER="${WBW_TIER:-pro}"
MAX_TIER="${WBW_MAX_TIER:-pro}"
ISSUER="${WBW_ISSUER:-radiant-garden-local}"
ALLOWED_TOOL_IDS_RAW="${WBW_ALLOWED_TOOL_IDS:-}"
DAYS_VALID="${WBW_DAYS_VALID:-7}"
CUSTOMER_ID="${WBW_CUSTOMER_ID:-john-local}"
OUT_DIR="${WBW_OUT_DIR:-./target/license_test}"

case "$SERVER_URL" in
    http://127.0.0.1:*|http://localhost:*|https://127.0.0.1:*|https://localhost:*)
        ;;
    *)
        if [[ "${ALLOW_NONLOCAL:-}" != "1" ]]; then
            echo "Error: refusing to issue against non-local server URL: $SERVER_URL" >&2
            echo "This helper is intended for local testing only." >&2
            echo "If you really intend to target a non-local environment, set ALLOW_NONLOCAL=1 explicitly." >&2
            exit 1
        fi
        echo "Warning: ALLOW_NONLOCAL=1 set; targeting non-local server URL: $SERVER_URL" >&2
        ;;
esac

mkdir -p "$OUT_DIR"
ENTITLEMENT_JSON_PATH="$OUT_DIR/entitlement.json"
PUBLIC_KEYS_JSON_PATH="$OUT_DIR/public_keys.json"
PY_SNIPPET_PATH="$OUT_DIR/python_bootstrap_snippet.py"

EXPIRES_AT_UNIX="$(python3 - <<'PY'
import os, time
try:
    days = int(os.environ.get("WBW_DAYS_VALID", "7"))
except Exception:
    days = 7
print(int(time.time()) + days * 24 * 3600)
PY
)"

ALLOWED_TOOL_IDS_JSON="$(python3 - <<'PY'
import json, os
raw = os.environ.get("WBW_ALLOWED_TOOL_IDS", "").strip()
if not raw:
    print("[]")
else:
    ids = [x.strip() for x in raw.split(",") if x.strip()]
    print(json.dumps(ids))
PY
)"

ISSUE_BODY="$(python3 - <<'PY'
import json, os
body = {
    "admin_secret": os.environ["WBW_ADMIN_SECRET"],
    "customer_id": os.environ.get("WBW_CUSTOMER_ID", "john-local"),
    "product": os.environ.get("WBW_PRODUCT", "whitebox_next_gen"),
    "tier": os.environ.get("WBW_TIER", "pro"),
    "max_tier": os.environ.get("WBW_MAX_TIER", "pro"),
    "allowed_tool_ids": json.loads(os.environ.get("ALLOWED_TOOL_IDS_JSON", "[]")),
    "expires_at_unix": int(os.environ["EXPIRES_AT_UNIX"]),
    "issuer": os.environ.get("WBW_ISSUER", "radiant-garden-local"),
}
print(json.dumps(body))
PY
)"

echo "Issuing entitlement from: $SERVER_URL/api/v2/entitlements/issue"
curl -fsS -X POST "$SERVER_URL/api/v2/entitlements/issue" \
  -H "Content-Type: application/json" \
  -d "$ISSUE_BODY" \
  > "$ENTITLEMENT_JSON_PATH"

echo "Fetching public keys from: $SERVER_URL/api/v2/public-keys"
curl -fsS "$SERVER_URL/api/v2/public-keys" > "$PUBLIC_KEYS_JSON_PATH"

KID="$(python3 - <<'PY' "$ENTITLEMENT_JSON_PATH"
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as f:
    ent = json.load(f)
kid = ent.get("kid")
if not kid:
    raise SystemExit("entitlement response missing kid")
print(kid)
PY
)"

PUBLIC_KEY_B64URL="$(python3 - <<'PY' "$PUBLIC_KEYS_JSON_PATH" "$KID"
import json, sys
keys_path, kid = sys.argv[1], sys.argv[2]
with open(keys_path, "r", encoding="utf-8") as f:
    data = json.load(f)
for item in data.get("keys", []):
    if item.get("kid") == kid:
        v = item.get("public_key_b64url")
        if not v:
            raise SystemExit("public_keys entry missing public_key_b64url")
        print(v)
        raise SystemExit(0)
raise SystemExit("no matching public key found for kid=" + kid)
PY
)"

cat > "$PY_SNIPPET_PATH" <<PY
import whitebox_workflows as wb

with open("$ENTITLEMENT_JSON_PATH", "r", encoding="utf-8") as f:
    signed_entitlement_json = f.read()

wbe = wb.WbEnvironment.from_signed_entitlement_json(
    signed_entitlement_json=signed_entitlement_json,
    public_key_kid="$KID",
    public_key_b64url="$PUBLIC_KEY_B64URL",
    include_pro=True,
    fallback_tier="open",
)

tools = wbe.list_tools()
print("Loaded tools:", len(tools))
print("First 10 tool IDs:", [t.get("id") for t in tools[:10]])
PY

echo ""
echo "Success. Artifacts written to: $OUT_DIR"
echo "- $ENTITLEMENT_JSON_PATH"
echo "- $PUBLIC_KEYS_JSON_PATH"
echo "- $PY_SNIPPET_PATH"
echo ""
echo "Run the generated Python snippet with your wbw_python environment to validate Pro startup."
