#!/usr/bin/env bash
# verify-agnos-sandbox.sh — Validate Mneme runs correctly inside AGNOS sandbox.
#
# Exit codes:
#   0 = all checks passed
#   1 = check failed

set -euo pipefail

PASS=0
FAIL=0

check() {
    local name="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        echo "  ✓ $name"
        ((PASS++))
    else
        echo "  ✗ $name"
        ((FAIL++))
    fi
}

echo "=== Mneme AGNOS Sandbox Verification ==="
echo ""

# 1. AGNOS environment
echo "1. Environment Detection"
check "AGNOS env vars" test -n "${AGNOS_VERSION:-}"
check "/run/agnos exists" test -d /run/agnos

# 2. Binary accessibility
echo ""
echo "2. Binary Checks"
check "mneme-api binary" command -v mneme-api
check "mneme-mcp binary" command -v mneme-mcp
check "mneme-ui binary" command -v mneme-ui

# 3. Data directory
echo ""
echo "3. Data Directory"
DATA_DIR="${MNEME_VAULT_DIR:-$HOME/.local/share/mneme}"
check "Data dir exists" test -d "$DATA_DIR" || mkdir -p "$DATA_DIR"
check "Data dir writable" touch "$DATA_DIR/.sandbox-test" && rm "$DATA_DIR/.sandbox-test"

# 4. Daimon connectivity
echo ""
echo "4. Daimon Integration"
DAIMON_URL="${DAIMON_URL:-http://127.0.0.1:8090}"
check "Daimon reachable" curl -sf "$DAIMON_URL/health"

# 5. API server smoke test
echo ""
echo "5. API Server"
MNEME_BIND="127.0.0.1:13838"
MNEME_VAULT_DIR="$DATA_DIR" MNEME_BIND="$MNEME_BIND" mneme-api &
API_PID=$!
sleep 2
check "API health" curl -sf "http://$MNEME_BIND/health"
check "API notes endpoint" curl -sf "http://$MNEME_BIND/v1/notes"
kill $API_PID 2>/dev/null || true

# 6. MCP server
echo ""
echo "6. MCP Server"
check "MCP initialize" bash -c 'echo "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}" | timeout 5 mneme-mcp | grep -q protocolVersion'

# Summary
echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
exit $((FAIL > 0 ? 1 : 0))
