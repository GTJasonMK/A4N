build_frontend() {
    log_section "构建前端"
    ensure_frontend_dependencies
    # frontend/dist is shared with local app builds; package an isolated snapshot.
    mkdir -p "$CACHE_ROOT"
    local build_dist
    build_dist="$(cd "$CACHE_ROOT" && pwd)/frontend-dist"
    rm -rf -- "$build_dist"
    mkdir -p "$build_dist"
    (
        cd "$FRONTEND_DIR"
        VITE_A4N_BASE_PATH="$PUBLIC_BASE_PATH" \
        VITE_A4N_API_BASE_URL="$API_BASE_URL" \
        npm run build -- --outDir "$build_dist"
    )

    [[ -f "$build_dist/index.html" ]] || die "前端构建后未找到 index.html"
    if ! grep -q "$PUBLIC_BASE_PATH/assets/" "$build_dist/index.html"; then
        die "前端资源路径未使用 $PUBLIC_BASE_PATH/assets，请检查 Vite base 配置"
    fi
    FRONTEND_PACKAGE_DIST="$build_dist"
    log_ok "前端部署产物已生成: $build_dist"
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
RUN apt-get clean \\
    && rm -rf /var/lib/apt/lists/* \\
    && apt-get -o Acquire::Retries=3 -o Acquire::http::No-Cache=true update \\
    && apt-get -o Acquire::Retries=3 install -y --no-install-recommends \\
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

    local cargo_host_packages="" cargo_component_packages=""
    local package component_package is_component
    local -a component_packages=()
    while IFS= read -r component_package; do
        [[ -n "$component_package" ]] && component_packages+=("$component_package")
    done < <(runtime_component_packages)
    for package in "${BUILD_PACKAGES[@]}"; do
        is_component=false
        for component_package in "${component_packages[@]}"; do
            [[ "$package" == "$component_package" ]] && is_component=true
        done
        if $is_component; then
            cargo_component_packages+=" -p $package"
        else
            cargo_host_packages+=" -p $package"
        fi
    done
    log_info "Cargo 构建 ${#BUILD_PACKAGES[@]} 个 package；Host 静态 DB，Worker/Sidecar 动态 DB"

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
        bash -c "set -euo pipefail; rustc --version; getconf GNU_LIBC_VERSION; protoc --version; if [[ -n '${cargo_host_packages}' ]]; then cargo build --release${cargo_host_packages} --target-dir /cargo-target --locked ${offline_flag[*]}; fi; if [[ -n '${cargo_component_packages}' ]]; then RUSTFLAGS='-C prefer-dynamic' cargo build --release --config 'profile.release.lto=\"off\"'${cargo_component_packages} --target-dir /cargo-target --locked ${offline_flag[*]}; fi" &
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
    log_ok "Host、工具和共享组件宿主（${#SERVICE_NAMES[@]} 个 service 角色）构建完成"

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
        return
    fi

    if [[ "$DEPLOY_MODE" == full ]] || plan_has_payload static; then
        build_frontend
    else
        log_info "前端未变化，跳过静态资源构建"
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
    local spec binary
    for spec in "${TOOL_SPECS[@]}"; do
        binary="${spec#*:}"
        cp "$BUILD_TARGET_DIR/release/$binary" "$PACKAGE_DIR/bin/$binary"
    done
    local shared_runtime
    shared_runtime="$(find "$(rustc --print target-libdir)" -maxdepth 1 -name 'libstd-*.so' -print -quit)"
    [[ -n "$shared_runtime" && -f "$shared_runtime" ]] || die "Rust toolchain shared libstd was not found"
    cp -L "$shared_runtime" "$PACKAGE_DIR/bin/services/$(basename "$shared_runtime")"
    shared_runtime="$BUILD_TARGET_DIR/release/deps/liba4n_db.so"
    [[ -f "$shared_runtime" ]] || die "构建产物缺少共享数据库运行库: $shared_runtime"
    cp -L "$shared_runtime" "$PACKAGE_DIR/bin/services/liba4n_db.so"
    cp -L "$A4N_ONNXRUNTIME_ROOT/lib/libonnxruntime.so" "$PACKAGE_DIR/bin/services/libonnxruntime.so.1"
    if [[ "$DEPLOY_MODE" == full ]] || plan_has_payload static; then
        [[ -f "$FRONTEND_PACKAGE_DIST/index.html" ]] \
            || die "未找到前端部署产物: $FRONTEND_PACKAGE_DIST"
        cp -a "$FRONTEND_PACKAGE_DIST/." "$PACKAGE_DIR/static/"
        cmp -s "$FRONTEND_PACKAGE_DIST/index.html" "$PACKAGE_DIR/static/index.html" \
            || die "部署包中的 static/index.html 与前端部署产物不一致"
        local packaged_asset_path
        packaged_asset_path="$(grep -m1 -oE 'src=\"[^\"]+\.js\"' "$PACKAGE_DIR/static/index.html" \
            | sed -n 's/^src=\"\([^\"]*\)\"$/\1/p')"
        [[ "$packaged_asset_path" == "$PUBLIC_BASE_PATH/assets/"* ]] \
            || die "部署包 JS asset 路径不是 $PUBLIC_BASE_PATH/assets: ${packaged_asset_path:-<missing>}"
    fi
    cp -a "$PROMPTS_DIR/." "$PACKAGE_DIR/prompts/"
    find "$PACKAGE_DIR/bin" -type f -exec chmod 755 {} +

    ruby "$RELEASE_MANIFEST_TOOL" generate \
        --package-root "$PACKAGE_DIR" \
        --target "$TARGET_PLATFORM" \
        --package "$HOST_PACKAGE" \
        --runtime-files "$(find "$PACKAGE_DIR/bin/services" -maxdepth 1 -name 'libstd-*.so' -print -quit | sed 's#^.*/##')",liba4n_db.so \
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
    log_ok "release manifest: ${#SERVICE_NAMES[@]} 个独立 service + ${#WORKER_NAMES[@]} 个独立 worker + ${#TOOL_SPECS[@]} 个工具 + 共享 Rust/ONNX Runtime"
    du -sh "$PACKAGE_PATH"
}

remote_shell_quote() {
    local value="$1"
    printf "'%s'" "${value//\'/\'\\\'\'}"
}

upload_package() {
    log_section "上传部署包"

    local remote_partial="${REMOTE_PACKAGE}.part"
    local remote_partial_arg
    local remote_package_arg
    local local_sum
    local remote_sum
    local rsync_ssh
    local remote_rsync=false
    local ssh_status
    local probe_attempt
    local upload_attempt
    remote_partial_arg="$(remote_shell_quote "$remote_partial")"
    remote_package_arg="$(remote_shell_quote "$REMOTE_PACKAGE")"
    local_sum="$(cut -d' ' -f1 "$PACKAGE_PATH.sha256")"

    log_info "上传大小: ${PACKAGE_SIZE:-$(du -h "$PACKAGE_PATH" | cut -f1)}"
    if command -v rsync >/dev/null 2>&1; then
        for probe_attempt in 1 2 3; do
            if ssh "${SSH_OPTS[@]}" "$REMOTE" "command -v rsync >/dev/null 2>&1 || exit 42"; then
                remote_rsync=true
                break
            else
                ssh_status=$?
            fi
            [[ "$ssh_status" == 42 ]] && break
            if ((probe_attempt == 3)); then
                die "SSH 检查远端 rsync 失败（退出码 $ssh_status），停止上传"
            fi
            log_warn "SSH 检查远端 rsync 失败（退出码 $ssh_status），5 秒后重试 $probe_attempt/3"
            sleep 5
        done
    fi

    if [[ "$remote_rsync" == true ]]; then
        printf -v rsync_ssh '%q ' ssh "${SSH_OPTS[@]}"
        ssh "${SSH_OPTS[@]}" "$REMOTE" \
            "if [[ ! -e $remote_partial_arg && -f $remote_package_arg ]]; then cp -a -- $remote_package_arg $remote_partial_arg; fi"
        log_info "上传方式: rsync（复用上次部署包，支持断点续传；下面显示实时进度）"
        for upload_attempt in 1 2 3; do
            if rsync \
                --human-readable \
                --info=progress2 \
                --partial \
                --rsh="$rsync_ssh" \
                "$PACKAGE_PATH" "$REMOTE:$remote_partial"; then
                break
            else
                ssh_status=$?
            fi
            if ((upload_attempt == 3)); then
                return "$ssh_status"
            fi
            log_warn "rsync 上传中断（退出码 $ssh_status），5 秒后重试 $upload_attempt/3"
            sleep 5
        done
    else
        log_info "上传方式: SSH 流式传输（下面显示已传输字节数）"
        dd if="$PACKAGE_PATH" bs=4M status=progress \
            | ssh "${SSH_OPTS[@]}" "$REMOTE" "cat > $remote_partial_arg"
    fi

    remote_sum="$(ssh "${SSH_OPTS[@]}" "$REMOTE" "sha256sum $remote_partial_arg | cut -d' ' -f1")"
    if [[ "$local_sum" != "$remote_sum" ]]; then
        die "上传包校验和不一致，临时文件已保留以便继续上传: $remote_partial"
    fi
    ssh "${SSH_OPTS[@]}" "$REMOTE" "mv -f -- $remote_partial_arg $remote_package_arg"
    log_ok "上传完成并通过 SHA256 校验"
}
