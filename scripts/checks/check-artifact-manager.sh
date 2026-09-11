#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
mkdir -p "$project_root/target/tmp"
fixture="$(mktemp -d "$project_root/target/tmp/artifact-wrapper.XXXXXX")"
trap 'rm -rf -- "$fixture"' EXIT
mkdir -p "$fixture/bin"

cat > "$fixture/bin/cargo" <<'EOF'
#!/bin/bash
printf '%s\n' "$CARGO_TARGET_DIR" "$@"
exit "${FAKE_CARGO_EXIT:-0}"
EOF
cat > "$fixture/artifact manager" <<'EOF'
#!/bin/bash
printf '%s\n' "$@"
exit 7
EOF
chmod +x "$fixture/bin/cargo" "$fixture/artifact manager"
ln -s "$(command -v bash)" "$fixture/bin/bash"
ln -s "$(command -v dirname)" "$fixture/bin/dirname"

status=0
PATH="$fixture/bin:$PATH" CARGO_TARGET_DIR="$fixture/custom target" \
    CARGO_ARTIFACTS_DISABLED=1 CARGO_ARTIFACTS_BIN=/missing-manager FAKE_CARGO_EXIT=9 \
    "$project_root/scripts/cargo.sh" test -- --nocapture > "$fixture/disabled" || status=$?
[[ "$status" == 9 ]] || { echo 'Cargo exit status was lost' >&2; exit 1; }
printf '%s\n' "$fixture/custom target" test -- --nocapture > "$fixture/expected"
cmp "$fixture/disabled" "$fixture/expected"

status=0
CARGO_ARTIFACTS_DISABLED=0 CARGO_ARTIFACTS_BIN="$fixture/artifact manager" \
    "$project_root/scripts/cargo.sh" check --target-dir "$fixture/custom target" > "$fixture/managed" || status=$?
[[ "$status" == 7 ]] || { echo 'manager failure was hidden' >&2; exit 1; }
printf '%s\n' run -- check --target-dir "$fixture/custom target" > "$fixture/expected"
cmp "$fixture/managed" "$fixture/expected"

PATH="$fixture/bin" CARGO_TARGET_DIR="$fixture/custom target" CARGO_ARTIFACTS_DISABLED=0 CARGO_ARTIFACTS_BIN= \
    "$project_root/scripts/cargo.sh" check > "$fixture/native" 2> "$fixture/warning"
[[ -s "$fixture/warning" ]] || { echo 'missing manager was silently ignored' >&2; exit 1; }
printf '%s\n' "$fixture/custom target" check > "$fixture/expected"
cmp "$fixture/native" "$fixture/expected"

printf '%s\n' 'artifact wrapper preserves arguments, target directory, failures, and explicit opt-out'
