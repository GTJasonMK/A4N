build_frontend() {
    log_section "构建前端"
    ensure_frontend_dependencies
    (
        cd "$FRONTEND_DIR"
        VITE_A4N_BASE_PATH="$PUBLIC_BASE_PATH" \
        VITE_A4N_API_BASE_URL="$API_BASE_URL" \
        npm run build
    )

    [[ -f "$FRONTEND_DIST/index.html" ]] || die "前端构建后未找到 index.html"
    if ! grep -q "$PUBLIC_BASE_PATH/assets/" "$FRONTEND_DIST/index.html"; then
        die "前端资源路径未使用 $PUBLIC_BASE_PATH/assets，请检查 Vite base 配置"
    fi
    log_ok "前端 dist 已生成: $FRONTEND_DIST"
}
plan_has_payload() {
    local requested="$1" payload
    for payload in "${PLAN_PAYLOADS[@]}"; do
        [[ "$payload" == "$requested" ]] && return 0
    done
    return 1
}

write_builder_dockerfile() {
    mkdir -p "$CACHE_ROOT"
    cat > "$CACHE_ROOT/Dockerfile.noble-builder" <<DOCKERFILE
FROM $RUST_BASE_IMAGE AS rust
FROM $UBUNTU_BASE_IMAGE
ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_TOOLCHAIN=1.95.0-x86_64-unknown-linux-gnu
ENV PATH=/usr/local/rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup
RUN apt-get update \\
    && apt-get install -y --no-install-recommends \\
       build-essential pkg-config libssl-dev ca-certificates cmake git curl protobuf-compiler libprotobuf-dev \\
       liblzma-dev zlib1g-dev libbz2-dev libzstd-dev \\
    && rm -rf /var/lib/apt/lists/*
RUN rustc --version \\
    && cargo --version \\
    && protoc --version \\
    && test -f /usr/include/google/protobuf/empty.proto \\
    && getconf GNU_LIBC_VERSION
DOCKERFILE
}

ensure_builder_image() {
    log_section "检查本地构建镜像"
    if $REBUILD_BUILDER || ! docker image inspect "$BUILDER_IMAGE" >/dev/null 2>&1; then
        log_info "构建 Docker 镜像: $BUILDER_IMAGE"
        write_builder_dockerfile
        docker build -t "$BUILDER_IMAGE" - < "$CACHE_ROOT/Dockerfile.noble-builder"
    else
        log_ok "构建镜像已存在: $BUILDER_IMAGE"
    fi

    docker run --rm "$BUILDER_IMAGE" bash -c \
        'rustc --version; cargo --version; protoc --version; getconf GNU_LIBC_VERSION; test -f /usr/include/google/protobuf/empty.proto'
}

build_backend() {
    log_section "本地容器构建后端"
    ensure_builder_image
    mkdir -p "$BUILD_TARGET_DIR" "$BUILD_CARGO_HOME/registry" "$BUILD_CARGO_HOME/git"

    local offline_flag=()
    local host_cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    local docker_cache_mounts=(
        -v "$A4N_ONNXRUNTIME_ROOT:/opt/a4n-onnxruntime:ro"
        -v "$BUILD_CARGO_HOME:/cargo-home"
    )
    if [[ "$CARGO_OFFLINE" == "true" ]]; then
        [[ -d "$host_cargo_home/registry" ]] || die "离线构建需要 $host_cargo_home/registry，或使用 --online"
        [[ -d "$host_cargo_home/git" ]] || die "离线构建需要 $host_cargo_home/git，或使用 --online"
        offline_flag=(--offline)
        docker_cache_mounts+=(
            -v "$host_cargo_home/registry:/cargo-home/registry:ro"
            -v "$host_cargo_home/git:/cargo-home/git:ro"
        )
        log_info "cargo 使用离线构建"
    else
        log_info "cargo 允许联网构建"
    fi

    local cargo_packages=""
    local package
    for package in "${BUILD_PACKAGES[@]}"; do
        cargo_packages+=" -p $package"
    done
    log_info "Cargo 构建 ${#BUILD_PACKAGES[@]} 个 package，实时显示编译进度"

    rm -f -- "$DOCKER_BUILD_CIDFILE"
    docker run --rm -t --cidfile "$DOCKER_BUILD_CIDFILE" \
        --user "$(id -u):$(id -g)" \
        -e CARGO_HOME=/cargo-home \
        -e RUSTUP_HOME=/usr/local/rustup \
        -e CARGO_BUILD_JOBS="${A4N_DEPLOY_CARGO_JOBS:-2}" \
        -e CARGO_TERM_PROGRESS_WHEN=always \
        -e CARGO_TERM_PROGRESS_WIDTH="${A4N_DEPLOY_CARGO_PROGRESS_WIDTH:-100}" \
        -e CARGO_TERM_COLOR=always \
        -e ORT_LIB_PATH=/opt/a4n-onnxruntime/lib \
        -e ORT_PREFER_DYNAMIC_LINK=1 \
        -e LD_LIBRARY_PATH=/opt/a4n-onnxruntime/lib \
        "${docker_cache_mounts[@]}" \
        -v "$PROJECT_ROOT:/work:ro" \
        -v "$BUILD_TARGET_DIR:/cargo-target" \
        -w /work \
        "$BUILDER_IMAGE" \
        bash -c "set -euo pipefail; rustc --version; getconf GNU_LIBC_VERSION; protoc --version; cargo build --release${cargo_packages} --target-dir /cargo-target --locked ${offline_flag[*]}" &
    DOCKER_BUILD_PID=$!
    local build_status
    if wait "$DOCKER_BUILD_PID"; then
        build_status=0
    else
        build_status=$?
    fi
    DOCKER_BUILD_PID=""
    ((build_status == 0)) || return "$build_status"
    rm -f -- "$DOCKER_BUILD_CIDFILE"
    DOCKER_BUILD_CIDFILE=""

    local binary
    while IFS= read -r binary; do
        [[ -x "$binary" ]] || die "后端构建后未找到二进制: $binary"
    done < <(required_binary_paths)
    log_ok "Host、工具和 ${#SERVICE_BINARIES[@]} 个 sidecar 构建完成"

}

verify_backend_binaries() {
    log_section "校验后端二进制"
    local binary max_glibc
    while IFS= read -r binary; do
        [[ -x "$binary" ]] || die "后端二进制不存在: $binary"
        file "$binary"
        max_glibc=$(strings "$binary" | grep -E '^GLIBC_[0-9]' | sort -V | tail -1 || true)
        [[ -n "$max_glibc" ]] || die "无法识别二进制 GLIBC 依赖: $binary"
        log_ok "$(basename "$binary") 最高 GLIBC 依赖: $max_glibc"
        if [[ "$(printf '%s\nGLIBC_2.39\n' "$max_glibc" | sort -V | tail -1)" != "GLIBC_2.39" ]]; then
            die "$(basename "$binary") 依赖 $max_glibc，高于线上 Ubuntu 24.04 的 GLIBC_2.39"
        fi
    done < <(required_binary_paths)
    [[ -r "$A4N_ONNXRUNTIME_ROOT/lib/libonnxruntime.so" ]] || die "缺少 ONNX Runtime 动态库"
    log_ok "ONNX Runtime $(onnxruntime_version) 已就绪"
}

build_all() {
    [[ "$DEPLOY_MODE" == noop ]] && {
        log_ok "没有新的应用变更，跳过构建"
        return
    }
    if $SKIP_BUILD; then
        log_warn "跳过构建，使用现有产物"
        local binary
        while IFS= read -r binary; do
            [[ -x "$binary" ]] || die "未找到后端二进制: $binary"
        done < <(required_binary_paths)
        [[ -f "$FRONTEND_DIST/index.html" ]] || die "未找到前端 dist: $FRONTEND_DIST"
        return
    fi

    if [[ "$DEPLOY_MODE" == full ]] || plan_has_payload static; then
        build_frontend
    else
        log_info "前端未变化，复用现有 frontend/dist"
    fi
    if ((${#BUILD_PACKAGES[@]} > 0)); then
        build_backend
    else
        log_info "没有受影响的 Rust package，复用现有二进制"
    fi
}

prepare_package() {
    log_section "制作部署包"
    rm -rf "$PACKAGE_DIR" "$DELTA_DIR" "$PACKAGE_PATH"
    mkdir -p "$PACKAGE_DIR/bin/services" "$PACKAGE_DIR/static" "$PACKAGE_DIR/prompts"

    cp "$BINARY_PATH" "$PACKAGE_DIR/bin/a4n-server"
    cp "$CONTROL_BINARY_PATH" "$PACKAGE_DIR/bin/a4n-control"
    cp "$AGENT_WORKER_BINARY_PATH" "$PACKAGE_DIR/bin/a4n-agent-worker"
    cp "$MIGRATION_BINARY_PATH" "$PACKAGE_DIR/bin/a4n-migration"
    local spec binary
    for spec in "${TOOL_SPECS[@]}"; do
        binary="${spec#*:}"
        cp "$BUILD_TARGET_DIR/release/$binary" "$PACKAGE_DIR/bin/$binary"
    done
    for binary in "${SERVICE_BINARIES[@]}"; do
        cp "$BUILD_TARGET_DIR/release/$binary" "$PACKAGE_DIR/bin/services/$binary"
    done
    cp -L "$A4N_ONNXRUNTIME_ROOT/lib/libonnxruntime.so" "$PACKAGE_DIR/bin/services/libonnxruntime.so.1"
    cp -a "$FRONTEND_DIST/." "$PACKAGE_DIR/static/"
    cp -a "$PROMPTS_DIR/." "$PACKAGE_DIR/prompts/"
    find "$PACKAGE_DIR/bin" -type f -exec chmod 755 {} +

    ruby "$RELEASE_MANIFEST_TOOL" generate \
        --package-root "$PACKAGE_DIR" \
        --target "$TARGET_PLATFORM" \
        --package "$HOST_PACKAGE" \
        --output "$PACKAGE_DIR/release-manifest.json"
    ruby "$RELEASE_MANIFEST_TOOL" verify \
        --package-root "$PACKAGE_DIR" \
        --manifest "$PACKAGE_DIR/release-manifest.json"
    (cd "$PACKAGE_DIR" && sha256sum -c release-manifest.sha256 >/dev/null)
    RELEASE_ID="$(sha256sum "$PACKAGE_DIR/release-manifest.json" | cut -d' ' -f1)"

    if [[ "$DEPLOY_MODE" == delta ]]; then
        mkdir -p "$DELTA_DIR"
        cp "$PACKAGE_DIR/release-manifest.json" "$DELTA_DIR/release-manifest.json"
        cp "$PACKAGE_DIR/release-manifest.sha256" "$DELTA_DIR/release-manifest.sha256"
        ruby "$DEPLOY_PLAN_TOOL" delta \
            --plan "$DEPLOY_PLAN_PATH" \
            --manifest "$PACKAGE_DIR/release-manifest.json" \
            --output "$DELTA_MANIFEST_PATH"
        local relative
        while IFS= read -r relative; do
            [[ -n "$relative" ]] || continue
            mkdir -p "$DELTA_DIR/$(dirname "$relative")"
            cp -a "$PACKAGE_DIR/$relative" "$DELTA_DIR/$relative"
        done < <(printf '%s\n' "${PLAN_ARTIFACT_PATHS[@]}")
        if plan_has_payload static; then
            cp -a "$PACKAGE_DIR/static" "$DELTA_DIR/static"
        fi
        tar -C "$DELTA_DIR" -czf "$PACKAGE_PATH" .
    else
        tar -C "$PACKAGE_DIR" -czf "$PACKAGE_PATH" .
    fi
    sha256sum "$PACKAGE_PATH" > "$PACKAGE_PATH.sha256"
    PACKAGE_SHA256="$(cut -d' ' -f1 "$PACKAGE_PATH.sha256")"
    PACKAGE_CREATED_AT="$(date --iso-8601=seconds)"
    PACKAGE_SIZE="$(du -h "$PACKAGE_PATH" | cut -f1)"

    log_ok "部署包: $PACKAGE_PATH"
    log_ok "SHA256: $PACKAGE_SHA256"
    log_ok "部署模式: $DEPLOY_MODE"
    log_ok "release manifest: ${#SERVICE_BINARIES[@]} 个 sidecar + ${#TOOL_SPECS[@]} 个工具 + ONNX Runtime"
    du -sh "$PACKAGE_PATH"
}

upload_package() {
    log_section "上传部署包"
    scp "${SSH_OPTS[@]}" "$PACKAGE_PATH" "$REMOTE:$REMOTE_PACKAGE"

    local local_sum
    local_sum=$(cut -d' ' -f1 "$PACKAGE_PATH.sha256")
    local remote_sum
    remote_sum=$(ssh "${SSH_OPTS[@]}" "$REMOTE" "sha256sum '$REMOTE_PACKAGE' | cut -d' ' -f1")
    [[ "$local_sum" == "$remote_sum" ]] || die "上传包校验和不一致"
    log_ok "上传完成并通过 SHA256 校验"
}
