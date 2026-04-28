#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AIKEN_BIN="${AIKEN_BIN:-aiken}"
if ! command -v "$AIKEN_BIN" >/dev/null 2>&1; then
  echo "Aiken CLI not found. Install Aiken or set AIKEN_BIN=/path/to/aiken." >&2
  exit 1
fi

command -v node >/dev/null 2>&1 || {
  echo "Node.js is required for bundle validation and generated off-chain checks." >&2
  exit 1
}
command -v npm >/dev/null 2>&1 || {
  echo "npm is required for generated off-chain checks." >&2
  exit 1
}

TMP_ROOT="${TMPDIR:-/tmp}/aikenflow-example-smoke-$$"
trap 'rm -rf "$TMP_ROOT"' EXIT
mkdir -p "$TMP_ROOT"

cargo build -q -p aikenflow-cli
CLI="$ROOT/target/debug/aikenflow"

examples=(
  simple-vault
  auction-lite
  programmable-token-mini
)

for example in "${examples[@]}"; do
  spec="$ROOT/examples/$example/protocol.yaml"
  out="$TMP_ROOT/$example"
  generated="$out/generated"

  "$CLI" check "$spec" --json > "$out-check.json"
  "$CLI" export "$spec" --out "$out-bundle.json"
  "$CLI" gen all "$spec" --out "$generated"
  "$CLI" audit "$spec" --out "$out-AUDIT.md"

  node -e "
    const fs = require('node:fs');
    const check = JSON.parse(fs.readFileSync('$out-check.json', 'utf8'));
    const bundle = JSON.parse(fs.readFileSync('$out-bundle.json', 'utf8'));
    if (check.ok !== true) throw new Error('$example check did not pass');
    if (bundle.schemaVersion !== '0.1.0') throw new Error('$example bundle schema mismatch');
    if (!bundle.graph?.states?.length) throw new Error('$example exported no graph states');
    if (!bundle.graph?.transactions?.length) throw new Error('$example exported no transactions');
    if (!bundle.artefacts?.aiken?.length) throw new Error('$example exported no Aiken artefacts');
  "

  (
    cd "$generated/offchain"
    npm install --silent
    npm run check
  )

  (
    cd "$generated/contracts"
    "$AIKEN_BIN" check
  )

  echo "example smoke passed: $example"
done
