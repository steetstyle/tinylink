#!/usr/bin/env bash
# Fetch a real ASP.NET Core solution to try tinylink against.
#
# unicpeak-analytics-api is a large multi-service solution using minimal-API
# endpoints — 1667 C# files across 92 projects — which makes it a good check
# that route extraction and code mapping work on realistic input.
#
# Usage:  bash example-projects/fetch.sh [--update]

set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="unicpeak-analytics-api"
URL="https://github.com/unicpeak/unicpeak-analytics-api.git"
TARGET="$DIR/$REPO"

if [ -d "$TARGET" ]; then
    if [ "${1:-}" = "--update" ]; then
        echo "→ Updating existing checkout"
        git -C "$TARGET" pull --ff-only
    else
        echo "→ Already present: $TARGET"
        echo "  (pass --update to pull newer commits)"
        exit 0
    fi
else
    echo "→ Cloning $URL"
    git clone --depth 1 "$URL" "$TARGET"
fi

echo
echo "Ready. Example commands:"
echo
R="example-projects/$REPO"
echo "  # routes found per service"
echo "  for s in Agency.API Billing.API Analytics.API Dashboard.API; do \\"
echo "    echo \"\$s: \$(cargo run -q -- route $R/\$s 2>/dev/null | grep -cE '^\\s+(GET|POST|PUT|DELETE|PATCH)')\""
echo "  done"
echo
echo "  # code graph for one service"
echo "  cargo run -q -- code $R/Agency.API | head -40"
echo
echo "  # PDG for a minimal-API handler"
echo "  cargo run -q -- pdg $R/Agency.API --class GetAgencyMembersEndpoint --method Handler"
echo
echo "  # link a trace against the service"
echo "  cargo run -q -- link tests/fixtures/otel/error_request.json $R/Agency.API --format dot"
