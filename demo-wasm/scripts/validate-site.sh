#!/usr/bin/env bash
# validate-site.sh — Verify assembled site before deployment.
# Usage: bash demo-wasm/scripts/validate-site.sh <site-dir>
set -euo pipefail

SITE="${1:?Usage: validate-site.sh <site-dir>}"
ERRORS=0

err() { echo "ERROR: $*" >&2; ERRORS=$((ERRORS + 1)); }

# --- 1. Required files ---
REQUIRED=(
    index.html
    style.css
    main.js
    renderer3d.js
    pkg/wavfc_demo.js
    pkg/wavfc_demo_bg.wasm
)
echo "=== Checking required files ==="
for f in "${REQUIRED[@]}"; do
    if [[ ! -f "$SITE/$f" ]]; then
        err "Missing required file: $f"
    else
        echo "  OK  $f"
    fi
done

# --- 2. Import map present ---
echo "=== Checking import map ==="
if grep -q '"importmap"' "$SITE/index.html" 2>/dev/null; then
    if grep -q '"three"' "$SITE/index.html" 2>/dev/null; then
        echo "  OK  import map maps \"three\""
    else
        err "import map found but does not map \"three\""
    fi
else
    err "index.html is missing <script type=\"importmap\">"
fi

# --- 3. Local JS imports resolve ---
echo "=== Checking local JS imports ==="
for jsfile in "$SITE"/*.js; do
    [[ -f "$jsfile" ]] || continue
    dir=$(dirname "$jsfile")
    # Extract relative import paths: from './...' or from '../...'
    imports=$(grep -oE "from ['\"](\./|\.\./)[^'\"]+['\"]" "$jsfile" 2>/dev/null \
        | sed "s/from ['\"]//;s/['\"]$//" || true)
    [[ -z "$imports" ]] && continue
    while IFS= read -r imp; do
        target="$dir/$imp"
        if [[ ! -f "$target" ]]; then
            err "$(basename "$jsfile"): import '$imp' -> file not found ($target)"
        else
            echo "  OK  $(basename "$jsfile"): $imp"
        fi
    done <<< "$imports"
done

# --- 4. Bare specifiers covered by import map ---
echo "=== Checking bare specifier coverage ==="
# Extract import map keys from index.html
IMPORT_MAP_KEYS=$(sed -n '/"imports"/,/}/p' "$SITE/index.html" 2>/dev/null \
    | grep -oE '"[^"]+"\s*:' | sed 's/"\(.*\)"\s*:/\1/' | grep -v '^imports$' || true)

for jsfile in "$SITE"/*.js; do
    [[ -f "$jsfile" ]] || continue
    # Extract all import specifiers, then filter to bare ones
    all_specs=$(grep -oE "from ['\"][^'\"]+['\"]" "$jsfile" 2>/dev/null \
        | sed "s/from ['\"]//;s/['\"]$//" || true)
    [[ -z "$all_specs" ]] && continue
    bare_specs=$(echo "$all_specs" | grep -v '^\.\.\?/' | grep -v '^https\?://' || true)
    [[ -z "$bare_specs" ]] && continue
    while IFS= read -r spec; do
        covered=false
        for key in $IMPORT_MAP_KEYS; do
            if [[ "$spec" == "$key" ]] || [[ "$spec" == "$key"* ]]; then
                covered=true
                break
            fi
        done
        if [[ "$covered" == false ]]; then
            err "$(basename "$jsfile"): bare specifier '$spec' not covered by import map"
        else
            echo "  OK  $(basename "$jsfile"): $spec"
        fi
    done <<< "$bare_specs"
done

# --- Summary ---
echo ""
if [[ $ERRORS -gt 0 ]]; then
    echo "FAILED: $ERRORS error(s) found."
    exit 1
else
    echo "All checks passed."
    exit 0
fi
