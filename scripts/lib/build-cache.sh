#!/usr/bin/env bash

create_build_cache() {
    local directory="$1" manager="${CARGO_ARTIFACTS_BIN:-cargo-artifacts}"
    if [[ -n "${CARGO_ARTIFACTS_BIN:-}" ]] || command -v "$manager" >/dev/null 2>&1; then
        "$manager" prepare --target-dir "$directory" >&2
    else
        mkdir -- "$directory" || return
        printf 'A4N: cargo-artifacts 未安装，临时缓存仍会在结束时回收，但未隔离文件系统快照。\n' >&2
    fi
}

make_build_cache() {
    local directory
    directory="$(mktemp -d "$1")" || return
    rmdir -- "$directory" || return
    create_build_cache "$directory" || return
    printf '%s\n' "$directory"
}

clean_build_cache() {
    local directory="$1" manifest="$2"
    [[ -e "$directory" || -L "$directory" ]] || return 0
    if [[ ! -d "$directory" || -L "$directory" ]]; then
        printf 'A4N: refusing changed cache path: %s\n' "$directory" >&2
        return 1
    fi
    # Cargo owns the build locks and knows how to remove its entire temporary target.
    CARGO_TARGET_DIR="$directory" CARGO_BUILD_BUILD_DIR="$directory" \
        cargo clean --manifest-path "$manifest" --target-dir "$directory" || return
    if [[ -e "$directory" ]]; then
        printf 'A4N: cargo clean left temporary cache behind: %s\n' "$directory" >&2
        return 1
    fi
}
