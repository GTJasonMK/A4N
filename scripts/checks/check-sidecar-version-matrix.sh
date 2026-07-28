#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

for command in cargo curl git python3 rg tar timeout; do
    command -v "$command" >/dev/null || {
        printf '%s is required for the sidecar version matrix\n' "$command" >&2
        exit 1
    }
done

legacy_commit=""
while IFS= read -r commit; do
    git cat-file -e "$commit:crates/a4n-export-service/Cargo.toml" 2>/dev/null || continue
    contract="$(git show "$commit:crates/a4n-service-protocol/src/health/contracts.rs" 2>/dev/null)" || continue
    if rg -q 'serde\(deny_unknown_fields\)' <<<"$contract" \
        && ! rg -q 'compatibility' <<<"$contract"; then
        legacy_commit="$commit"
        break
    fi
done < <(git rev-list --all)

if [[ -z "$legacy_commit" ]]; then
    printf 'no historical strict ServiceVersion commit with export sidecar was found\n' >&2
    exit 1
fi

matrix_dir="$(mktemp -d "${TMPDIR:-/tmp}/a4n-sidecar-version-matrix.XXXXXX")"
sidecar_pid=""
cleanup() {
    if [[ -n "$sidecar_pid" ]]; then
        kill "$sidecar_pid" 2>/dev/null || true
        wait "$sidecar_pid" 2>/dev/null || true
    fi
    rm -rf "$matrix_dir"
}
trap cleanup EXIT INT TERM

legacy_root="$matrix_dir/legacy"
mkdir -p "$legacy_root"
git archive "$legacy_commit" | tar -x -C "$legacy_root"

create_probe() {
    local probe_dir="$1"
    local source_root="$2"
    cp -R "$repo_root/scripts/checks/sidecar-version-probe" "$probe_dir"
    ln -s "$source_root" "$probe_dir/source"
}

current_probe="$matrix_dir/current-probe"
legacy_probe="$matrix_dir/legacy-probe"
create_probe "$current_probe" "$repo_root"
create_probe "$legacy_probe" "$legacy_root"

current_target="$matrix_dir/target-current"
legacy_target="$matrix_dir/target-legacy"
printf 'building current and historical export sidecars (%s)\n' "$legacy_commit"
CARGO_TARGET_DIR="$current_target" timeout 900 cargo build --locked -p a4n-export-service
CARGO_TARGET_DIR="$legacy_target" timeout 900 cargo build \
    --locked --manifest-path "$legacy_root/Cargo.toml" -p a4n-export-service
CARGO_TARGET_DIR="$current_target" timeout 600 cargo build --manifest-path "$current_probe/Cargo.toml"
CARGO_TARGET_DIR="$legacy_target" timeout 600 cargo build --manifest-path "$legacy_probe/Cargo.toml"

current_sidecar="$current_target/debug/a4n-export-service"
legacy_sidecar="$legacy_target/debug/a4n-export-service"
current_probe_bin="$current_target/debug/a4n-sidecar-version-probe"
legacy_probe_bin="$legacy_target/debug/a4n-sidecar-version-probe"

pick_addr() {
    python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(f"127.0.0.1:{s.getsockname()[1]}"); s.close()'
}

start_sidecar() {
    local binary="$1"
    local addr="$2"
    local log="$3"
    A4N_EXPORT_SERVICE_ADDR="$addr" \
        A4N_SIDECAR_AUTH_TOKEN="version-matrix-token" \
        "$binary" >"$log" 2>&1 &
    sidecar_pid=$!
    for _ in {1..100}; do
        if ! kill -0 "$sidecar_pid" 2>/dev/null; then
            wait "$sidecar_pid" || true
            sidecar_pid=""
            sed -n '1,160p' "$log" >&2
            return 1
        fi
        if curl --fail --silent --show-error --max-time 1 "http://$addr/healthz" >/dev/null 2>&1; then
            return
        fi
        sleep 0.1
    done
    printf 'sidecar did not become healthy: %s\n' "$binary" >&2
    sed -n '1,160p' "$log" >&2
    return 1
}

stop_sidecar() {
    kill "$sidecar_pid" 2>/dev/null || true
    wait "$sidecar_pid" 2>/dev/null || true
    sidecar_pid=""
}

run_probe() {
    local label="$1"
    local probe="$2"
    local addr="$3"
    local compatibility="$4"
    timeout 30 "$probe" "http://$addr" "$compatibility"
    printf '%s: passed\n' "$label"
}

addr="$(pick_addr)"
start_sidecar "$legacy_sidecar" "$addr" "$matrix_dir/legacy-sidecar.log"
run_probe 'current host -> legacy sidecar' "$current_probe_bin" "$addr" legacy
stop_sidecar

addr="$(pick_addr)"
start_sidecar "$current_sidecar" "$addr" "$matrix_dir/current-sidecar.log"
run_probe 'legacy host -> current sidecar' "$legacy_probe_bin" "$addr" legacy
run_probe 'current host -> current sidecar' "$current_probe_bin" "$addr" advertised
stop_sidecar

addr="$(pick_addr)"
start_sidecar "$legacy_sidecar" "$addr" "$matrix_dir/rollback-sidecar.log"
run_probe 'current host -> rollback sidecar' "$current_probe_bin" "$addr" legacy
stop_sidecar

printf 'real export sidecar version matrix passed with historical commit %s\n' "$legacy_commit"
