#!/usr/bin/env bash
# Refresh the optional Graphify code knowledge graph for Weft.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v graphify >/dev/null 2>&1; then
    if [ -x "$HOME/.local/bin/graphify" ]; then
        export PATH="$HOME/.local/bin:$PATH"
    else
        cat >&2 <<'EOF'
graphify is not installed.

  npm install -g graphifyy --prefix ~/.local
  export PATH="$HOME/.local/bin:$PATH"

Then rerun ./tools/graphify.sh
EOF
        exit 1
    fi
fi

if ! find crates -type f -name '*.rs' -print -quit 2>/dev/null | grep -q .; then
    echo "No Rust sources yet; the knowledge graph will be created once Phase 1 starts."
    exit 0
fi

graphify extract . --scope all --out graphify-out \
    --exclude '**/target/**' \
    --exclude '**/.git/**' \
    --exclude '**/.cargo/**' \
    --exclude '**/graphify-out/**' \
    --exclude '**/.graphify/**' \
    --exclude '**/node_modules/**' \
    --exclude '**/vendor/**' \
    --exclude '**/deploy/state/**' \
    --exclude '**/docs/**' \
    --exclude '*.md' \
    --exclude '*.toml' \
    --exclude '*.json' \
    --exclude '*.yaml' \
    --exclude '*.yml' \
    --exclude '*.lock' \
    --exclude 'LICENSE*'

cat <<'EOF'

Graph written to graphify-out/.graphify/graph.json
Query it with:

  graphify query graphify-out/.graphify/graph.json "where is upload validation"

EOF
