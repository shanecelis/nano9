#!/usr/bin/env bash
# Run `cargo run -- check` on all non-Rust examples (directories with Nano9.toml,
# and .p8lua / .lua / .p8 / .toml files) to ensure they don't have errors.
# A check fails if the process exits non-zero or if "ERROR" appears in the output.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXAMPLES_DIR="$REPO_ROOT/examples"
FAILED=0

cd "$REPO_ROOT"

# Collect non-Rust example paths:
# 1. Directories under examples/ that contain Nano9.toml
# 2. Files under examples/ with extensions .p8lua, .lua, .p8, .toml (exclude .rs)
PATHS=()
while IFS= read -r d; do
  [[ -n "$d" ]] && PATHS+=("$d")
done < <(find "$EXAMPLES_DIR" -mindepth 1 -maxdepth 1 -type d -exec test -f {}/Nano9.toml \; -print 2>/dev/null)

shopt -s nullglob
for ext in p8lua lua p8 toml; do
  for f in "$EXAMPLES_DIR"/*."$ext"; do
    [[ -f "$f" ]] && PATHS+=("$f")
  done
done
shopt -u nullglob

if [[ ${#PATHS[@]} -eq 0 ]]; then
  echo "No non-Rust examples found under $EXAMPLES_DIR"
  exit 0
fi

echo "Checking ${#PATHS[@]} non-Rust example(s)..."
for path in "${PATHS[@]}"; do
  rel="${path#$REPO_ROOT/}"
  printf '\n--- %s ---\n' "$rel"
  if [[ "$path" =~ \.toml$ ]] || [ -d "$path" ]; then
    output=$(cargo run --bin n9 --features "cli,scripting" -- check "$path" 2>&1)
  else
    output=$(NANO9_ASSETS_DIR=assets cargo run --bin n9 --features "cli,scripting" -- check "$path" 2>&1)
  fi
  exit_code=$?
  if [[ $exit_code -ne 0 ]]; then
    echo "$output"
    echo "  FAILED: $rel (exit code $exit_code)"
    FAILED=1
  elif echo "$output" | grep -q "ERROR"; then
    echo "$output"
    echo "  FAILED: $rel (errors in output)"
    FAILED=1
  else
    echo "  OK: $rel"
  fi
done

if [[ $FAILED -ne 0 ]]; then
  echo "One or more checks failed."
  exit 1
fi
echo "All checks passed."
