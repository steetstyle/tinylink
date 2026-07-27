#!/usr/bin/env bash
# ── tinylink Prometheus live integration test (eShopOnContainers) ──
# 1. Starts Docker Compose (Prometheus + eShop metrics exporter)
# 2. Waits for Prometheus to be ready
# 3. Validates Prometheus has scraped service graph metrics
# 4. Runs tinylink prometheus query (JSON + DOT)
# 5. Verifies output matches expected eShop services
# 6. Tears down
#
# Usage:  bash tests/docker/prometheus/test_live.sh

set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$DIR/../../.." && pwd)"
PROM_URL="http://localhost:9095"
COMPOSE_FILE="$DIR/docker-compose.yml"
PASS=0
FAIL=0

cleanup() {
    echo "→ Cleaning up Docker containers..."
    docker compose -f "$COMPOSE_FILE" down --remove-orphans 2>/dev/null || true
}
trap cleanup EXIT

check() {
    local label="$1"
    shift
    if "$@"; then
        echo "  ✓ $label"
        PASS=$((PASS + 1))
    else
        echo "  ✗ $label"
        FAIL=$((FAIL + 1))
    fi
}

# ── 1. Start Docker Compose ───────────────────────────────────────
echo "═══════════════════════════════════════════════════════════════"
echo "  tinylink — eShopOnContainers Prometheus live test"
echo "═══════════════════════════════════════════════════════════════"

echo ""
echo "── 1. Starting Docker Compose ──"
cleanup
docker compose -f "$COMPOSE_FILE" up -d

# ── 2. Wait for Prometheus readiness ──────────────────────────────
echo ""
echo "── 2. Waiting for Prometheus ──"
for i in $(seq 1 30); do
    if curl -sf "$PROM_URL/api/v1/status/config" >/dev/null 2>&1; then
        echo "     Ready after ${i}s"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "     ERROR: Prometheus did not start"
        docker compose -f "$COMPOSE_FILE" logs prometheus
        exit 1
    fi
    sleep 1
done

echo "     Waiting for scrape cycle..."
sleep 12

# ── 3. Verify Prometheus has service graph metrics ────────────────
echo ""
echo "── 3. Verify Prometheus scraped metrics ──"
DATA=$(curl -sf "$PROM_URL/api/v1/query?query=traces_service_graph_request_server_count" 2>/dev/null || echo "")
if echo "$DATA" | grep -q '"status":"success"'; then
    COUNT=$(echo "$DATA" | python3 -c "
import sys,json
d=json.loads(sys.stdin.read())
print(len(d.get('data',{}).get('result',[])))" 2>/dev/null || echo "0")
    echo "     Metric series: $COUNT"
    check "Prometheus has service graph metrics" [ "$COUNT" -ge 20 ]
else
    echo "     ERROR: Prometheus query failed"
    echo "$DATA" | head -5
    check "Prometheus has service graph metrics" false
fi

# ── 4. Build tinylink ────────────────────────────────────────────
echo ""
echo "── 4. Build tinylink ──"
cargo build --features online --quiet 2>&1
check "Build succeeds" true

TINYLINK="$ROOT/target/debug/tinylink"

# ── 5. JSON format query ─────────────────────────────────────────
echo ""
echo "── 5. tinylink prometheus query (JSON) ──"
JSON_OUTPUT=$("$TINYLINK" prometheus "$PROM_URL" --since 5m --format json 2>&1 || true)

# Validate JSON parseable
check "Valid JSON output" echo "$JSON_OUTPUT" | python3 -c "
import sys, json
json.loads(sys.stdin.read())
print('ok')
" 2>/dev/null | grep -q ok

# Check service nodes — should include all eShop services
NODES=$(echo "$JSON_OUTPUT" | python3 -c "
import sys, json
d = json.loads(sys.stdin.read())
print(len(d.get('nodes', {})))
" 2>/dev/null || echo "0")
echo "     → $NODES service nodes"
check "Has 10+ service nodes" [ "$NODES" -ge 10 ]

# Verify all core eShop services exist
for svc in \
    webmvc catalog.api ordering.api basket.api payment.api identity.api \
    catalog-db ordering-db basket-data identity-db rabbitmq; do
    check "Contains '$svc'" echo "$JSON_OUTPUT" | grep -q "\"$svc\""
done

# Check edges
EDGES=$(echo "$JSON_OUTPUT" | python3 -c "
import sys, json
d = json.loads(sys.stdin.read())
print(len(d.get('edges', [])))
" 2>/dev/null || echo "0")
echo "     → $EDGES edges"
check "Has 25+ edges" [ "$EDGES" -ge 25 ]

# ── 6. DOT format query ─────────────────────────────────────────
echo ""
echo "── 6. tinylink prometheus query (DOT) ──"
DOT_OUTPUT=$("$TINYLINK" prometheus "$PROM_URL" --since 5m --format dot 2>&1 || true)
check "Valid DOT output" echo "$DOT_OUTPUT" | grep -q "digraph"
check "DOT has service nodes" echo "$DOT_OUTPUT" | grep -q "catalog.api"
check "DOT has edges" echo "$DOT_OUTPUT" | grep -q "webmvc.*catalog.api"
check "DOT shows gRPC interaction" echo "$DOT_OUTPUT" | grep -q "BasketService\|ordering.api"

# ── 7. Edge count consistency ────────────────────────────────────
echo ""
echo "── 7. Consistency check ──"
JSON_EDGES=$(echo "$JSON_OUTPUT" | python3 -c "
import sys, json
d = json.loads(sys.stdin.read())
print(len(d.get('edges', [])))
" 2>/dev/null || echo "0")
DOT_EDGES=$(echo "$DOT_OUTPUT" | grep -c "\->" 2>/dev/null || echo "0")
check "JSON and DOT edge counts match ($JSON_EDGES ≈ $DOT_EDGES)" \
    [ "$((JSON_EDGES - DOT_EDGES))" -lt 3 ] && [ "$((DOT_EDGES - JSON_EDGES))" -lt 3 ]

# ── 8. Summary ───────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Results: $PASS passed, $FAIL failed"
echo "═══════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
