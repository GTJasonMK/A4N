#!/usr/bin/env bash
#=============================================================================
# A4N 云端一键部署脚本
#
# 用法:
#   ./scripts/deploy/deploy-cloud.sh
#   ./scripts/deploy/deploy-cloud.sh \
#     --host server.example.com \
#     --user ubuntu \
#     --ssh-key ~/.ssh/a4n-deploy \
#     --public-origin https://app.example.com
#   ./scripts/deploy/deploy-cloud.sh --online          # 容器内允许 cargo 联网拉取缺失依赖
#   ./scripts/deploy/deploy-cloud.sh --skip-build      # 使用已有 Host/工具/sidecar/ONNX Runtime 和 frontend/dist
#   ./scripts/deploy/deploy-cloud.sh --rebuild-builder # 重建本地 Ubuntu 24 构建镜像
#   ./scripts/deploy/deploy-cloud.sh --no-smoke        # 跳过公网 smoke test
#   ./scripts/deploy/deploy-cloud.sh --help
#
#   首次可将 scripts/deploy/deploy-cloud.env.example 复制为 .deploy.env 并填写目标，
#   之后直接运行本脚本即可；已有环境变量优先于 .deploy.env。
#
# 约束:
#   - 后端只在本地 Ubuntu 24 / glibc 2.39 Docker 容器中构建，不在 2G 云服务器上构建。
#   - 只上传服务器 Host、工具、完整 sidecar、ONNX Runtime、static 和 prompts，不覆盖远端 .env 与 data。
#   - 部署 Host、SSH key 和公网 origin 必须显式提供，根路径默认期望 404。
#
# 常用环境变量覆盖:
#   A4N_DEPLOY_ENV_FILE
#   A4N_DEPLOY_HOST, A4N_DEPLOY_USER, A4N_DEPLOY_SSH_KEY
#   A4N_DEPLOY_REMOTE_DIR, A4N_DEPLOY_SERVICE, A4N_DEPLOY_BASE_PATH
#   A4N_DEPLOY_API_BASE_URL, A4N_DEPLOY_PUBLIC_ORIGIN
#   A4N_DEPLOY_RECORD_DIR
#   A4N_DEPLOY_LOCK_FILE
#   A4N_DEPLOY_CARGO_OFFLINE=false
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
load_dotenv "${A4N_DEPLOY_ENV_FILE:-$PROJECT_ROOT/.deploy.env}"

FRONTEND_DIR="$PROJECT_ROOT/frontend"
FRONTEND_DIST="$FRONTEND_DIR/dist"
PROMPTS_DIR="$PROJECT_ROOT/prompts"

DEPLOY_HOST="${A4N_DEPLOY_HOST:-}"
DEPLOY_USER="${A4N_DEPLOY_USER:-ubuntu}"
SSH_KEY="${A4N_DEPLOY_SSH_KEY:-}"
REMOTE_DIR="${A4N_DEPLOY_REMOTE_DIR:-}"
REMOTE_PACKAGE="${A4N_DEPLOY_REMOTE_PACKAGE:-}"
REMOTE_SERVICE="${A4N_DEPLOY_SERVICE:-a4n.service}"

PUBLIC_BASE_PATH="${A4N_DEPLOY_BASE_PATH:-/a4n}"
PUBLIC_ORIGIN="${A4N_DEPLOY_PUBLIC_ORIGIN:-}"
API_BASE_URL="${A4N_DEPLOY_API_BASE_URL:-}"
ROOT_EXPECTED_STATUS="${A4N_DEPLOY_ROOT_EXPECTED_STATUS:-404}"

BUILDER_IMAGE="${A4N_DEPLOY_BUILDER_IMAGE:-a4n-rust-noble:1.95}"
RUST_BASE_IMAGE="${A4N_DEPLOY_RUST_BASE:-docker.m.daocloud.io/library/rust:1.95-bookworm}"
UBUNTU_BASE_IMAGE="${A4N_DEPLOY_UBUNTU_BASE:-docker.m.daocloud.io/library/ubuntu:24.04}"
BINARY_PATH="$CARGO_TARGET_DIR/release/a4n-server"
CONTROL_BINARY_PATH="$CARGO_TARGET_DIR/release/a4n-control"
AGENT_WORKER_BINARY_PATH="$CARGO_TARGET_DIR/release/a4n-agent-worker"
MIGRATION_BINARY_PATH="$CARGO_TARGET_DIR/release/a4n-migration"
HOST_PACKAGE="a4n-server"
TARGET_PLATFORM="linux-x86_64"
RELEASE_MANIFEST_TOOL="$PROJECT_ROOT/scripts/release_manifest.rb"
SERVICE_BINARIES=()
TOOL_SPECS=()
BUILD_PACKAGES=()

CARGO_OFFLINE="${A4N_DEPLOY_CARGO_OFFLINE:-true}"
SKIP_BUILD=false
REBUILD_BUILDER=false
RUN_SMOKE=true
KEEP_PACKAGE=false

CACHE_ROOT="$CARGO_TARGET_DIR/deploy-cloud"
PACKAGE_DIR="$CACHE_ROOT/package"
PACKAGE_PATH="$CACHE_ROOT/a4n-deploy.tar.gz"
PACKAGE_SHA256=""
PACKAGE_CREATED_AT=""
PACKAGE_SIZE=""
DEPLOY_RECORD_DIR="${A4N_DEPLOY_RECORD_DIR:-$PROJECT_ROOT/deploy-records}"
DEPLOY_RECORD_PATH=""
DEPLOY_STARTED_AT="$(date --iso-8601=seconds)"
DEPLOY_STARTED_EPOCH="$(date +%s)"
BUILD_ELAPSED_SECONDS=""
DEPLOY_LOCK_FILE="${A4N_DEPLOY_LOCK_FILE:-$CARGO_TARGET_DIR/deploy-cloud.lock}"
DEPLOY_LOCK_FD=""
DOCKER_BUILD_CIDFILE="$CACHE_ROOT/build-container.cid"
DOCKER_BUILD_PID=""

usage() {
    print_script_usage "$0" 23
}

acquire_deploy_lock() {
    command -v flock >/dev/null 2>&1 || die "未找到命令: flock"
    mkdir -p "$(dirname "$DEPLOY_LOCK_FILE")"
    exec {DEPLOY_LOCK_FD}>"$DEPLOY_LOCK_FILE"
    flock -n "$DEPLOY_LOCK_FD" || die "已有部署正在运行；锁文件: $DEPLOY_LOCK_FILE"
    cleanup_stale_build_container
}

cleanup_stale_build_container() {
    local cid image
    [[ -r "$DOCKER_BUILD_CIDFILE" ]] || return 0
    command -v docker >/dev/null 2>&1 || return 0
    cid="$(tr -d '[:space:]' < "$DOCKER_BUILD_CIDFILE")"
    if [[ "$cid" =~ ^[0-9a-f]{12,64}$ ]]; then
        image="$(docker inspect --format '{{.Config.Image}}' "$cid" 2>/dev/null || true)"
        if [[ "$image" == "$BUILDER_IMAGE" ]]; then
            docker rm -f "$cid" >/dev/null 2>&1 || true
        fi
    fi
    rm -f -- "$DOCKER_BUILD_CIDFILE"
}

load_release_inventory() {
    local package binary
    BUILD_PACKAGES=("$HOST_PACKAGE")
    SERVICE_BINARIES=()
    TOOL_SPECS=()

    while IFS=$'\t' read -r _service binary _rest; do
        [[ -n "$binary" ]] || continue
        SERVICE_BINARIES+=("$binary")
        BUILD_PACKAGES+=("$binary")
    done < <(sidecar_catalog_rows)

    while IFS=$'\t' read -r package binary; do
        [[ "$package" == "$HOST_PACKAGE" ]] && continue
        TOOL_SPECS+=("$package:$binary")
        BUILD_PACKAGES+=("$package")
    done < <(release_tool_rows)

    if [[ -n "$DEPLOY_RECORD_PATH" ]]; then
        {
            echo "sidecar_count=${#SERVICE_BINARIES[@]}"
            echo "sidecars=${SERVICE_BINARIES[*]}"
            echo "tool_count=${#TOOL_SPECS[@]}"
            echo "tools=${TOOL_SPECS[*]}"
        } >> "$DEPLOY_RECORD_PATH"
    fi
}

required_binary_paths() {
    printf '%s\n' "$BINARY_PATH" "$CONTROL_BINARY_PATH" "$AGENT_WORKER_BINARY_PATH" "$MIGRATION_BINARY_PATH"
    local spec binary
    for spec in "${TOOL_SPECS[@]}"; do
        binary="${spec#*:}"
        printf '%s/release/%s\n' "$CARGO_TARGET_DIR" "$binary"
    done
    for binary in "${SERVICE_BINARIES[@]}"; do
        printf '%s/release/%s\n' "$CARGO_TARGET_DIR" "$binary"
    done
}

format_elapsed() {
    local total_seconds="$1"
    printf '%02dh %02dm %02ds' \
        "$((total_seconds / 3600))" \
        "$((total_seconds % 3600 / 60))" \
        "$((total_seconds % 60))"
}

start_deploy_record() {
    command -v tee >/dev/null 2>&1 || die "未找到命令: tee"
    DEPLOY_STARTED_AT="$(date --iso-8601=seconds)"
    DEPLOY_STARTED_EPOCH="$(date +%s)"
    mkdir -p "$DEPLOY_RECORD_DIR"
    DEPLOY_RECORD_PATH="$DEPLOY_RECORD_DIR/deploy-$(date +%Y%m%d%H%M%S).log"
    exec > >(tee -a "$DEPLOY_RECORD_PATH") 2>&1
    {
        echo "started_at=$DEPLOY_STARTED_AT"
        echo "branch=$(git -C "$PROJECT_ROOT" branch --show-current 2>/dev/null || echo unknown)"
        echo "commit=$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
        echo "host=$REMOTE"
        echo "remote_dir=$REMOTE_DIR"
        echo "service=$REMOTE_SERVICE"
        echo "public_app_url=$PUBLIC_APP_URL"
        echo "ssh_key_path=$SSH_KEY"
        echo "cargo_offline=$CARGO_OFFLINE"
        echo "target=$TARGET_PLATFORM"
        echo "lock_file=$DEPLOY_LOCK_FILE"
    } >> "$DEPLOY_RECORD_PATH"
}

cleanup_deploy_resources() {
    local cid image
    if [[ -n "$DOCKER_BUILD_PID" ]] && kill -0 "$DOCKER_BUILD_PID" 2>/dev/null; then
        kill -TERM "$DOCKER_BUILD_PID" 2>/dev/null || true
        wait "$DOCKER_BUILD_PID" 2>/dev/null || true
    fi
    if [[ -n "$DOCKER_BUILD_CIDFILE" && -r "$DOCKER_BUILD_CIDFILE" ]]; then
        cid="$(tr -d '[:space:]' < "$DOCKER_BUILD_CIDFILE")"
        if [[ "$cid" =~ ^[0-9a-f]{12,64}$ ]]; then
            image="$(docker inspect --format '{{.Config.Image}}' "$cid" 2>/dev/null || true)"
            if [[ "$image" == "$BUILDER_IMAGE" ]]; then
                docker rm -f "$cid" >/dev/null 2>&1 || true
            fi
        fi
        rm -f -- "$DOCKER_BUILD_CIDFILE"
    fi
    DOCKER_BUILD_PID=""
    DOCKER_BUILD_CIDFILE=""
}

handle_deploy_signal() {
    exit "$1"
}

finish_deploy_record() {
    local status=$?
    local elapsed_seconds=$(( $(date +%s) - DEPLOY_STARTED_EPOCH ))
    cleanup_deploy_resources
    if [[ -n "$DEPLOY_RECORD_PATH" ]]; then
        {
            echo "finished_at=$(date --iso-8601=seconds)"
            echo "status=$([[ "$status" -eq 0 ]] && echo success || echo failed)"
            echo "exit_status=$status"
            echo "elapsed_seconds=$elapsed_seconds"
            [[ -n "$BUILD_ELAPSED_SECONDS" ]] && echo "build_elapsed_seconds=$BUILD_ELAPSED_SECONDS"
            [[ -n "$PACKAGE_CREATED_AT" ]] && echo "package_created_at=$PACKAGE_CREATED_AT"
            [[ -n "$PACKAGE_SIZE" ]] && echo "package_size=$PACKAGE_SIZE"
            [[ -n "$PACKAGE_SHA256" ]] && echo "package_sha256=$PACKAGE_SHA256"
        } >> "$DEPLOY_RECORD_PATH"
    else
        exit "$status"
    fi
    if [[ "$status" -eq 0 ]]; then
        log_ok "本次部署耗时: $(format_elapsed "$elapsed_seconds")"
    else
        log_warn "部署失败，耗时: $(format_elapsed "$elapsed_seconds")"
    fi
    exit "$status"
}

trap finish_deploy_record EXIT
trap 'handle_deploy_signal 130' INT
trap 'handle_deploy_signal 143' TERM
trap 'handle_deploy_signal 129' HUP

normalize_base_path() {
    local value="$1"
    if [[ -z "$value" || "$value" == "/" ]]; then
        printf '/'
        return
    fi
    value="/${value#/}"
    value="${value%/}"
    printf '%s' "$value"
}

normalize_api_base_url() {
    local value="$1"
    if [[ -z "$value" ]]; then
        printf '%s/api' "$PUBLIC_BASE_PATH"
        return
    fi
    printf '%s' "${value%/}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--host 需要参数"
            DEPLOY_HOST="$2"
            shift 2
            ;;
        --user)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--user 需要参数"
            DEPLOY_USER="$2"
            shift 2
            ;;
        --ssh-key)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--ssh-key 需要参数"
            SSH_KEY="$2"
            shift 2
            ;;
        --remote-dir)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--remote-dir 需要参数"
            REMOTE_DIR="$2"
            shift 2
            ;;
        --service)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--service 需要参数"
            REMOTE_SERVICE="$2"
            shift 2
            ;;
        --base-path)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--base-path 需要参数"
            PUBLIC_BASE_PATH="$2"
            shift 2
            ;;
        --api-base-url)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--api-base-url 需要参数"
            API_BASE_URL="$2"
            shift 2
            ;;
        --public-origin)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--public-origin 需要参数"
            PUBLIC_ORIGIN="$2"
            shift 2
            ;;
        --online)
            CARGO_OFFLINE=false
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --rebuild-builder)
            REBUILD_BUILDER=true
            shift
            ;;
        --no-smoke)
            RUN_SMOKE=false
            shift
            ;;
        --keep-package)
            KEEP_PACKAGE=true
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            die "未知参数: $1"
            ;;
    esac
done

$SKIP_BUILD && $REBUILD_BUILDER && die "--skip-build 与 --rebuild-builder 不能同时使用"
$SKIP_BUILD && [[ "$CARGO_OFFLINE" == "false" ]] && die "--skip-build 与 --online/A4N_DEPLOY_CARGO_OFFLINE=false 不能同时使用"

[[ -n "$DEPLOY_HOST" ]] || die "必须通过 --host 或 A4N_DEPLOY_HOST 指定部署 Host"
[[ -n "$SSH_KEY" ]] || die "必须通过 --ssh-key 或 A4N_DEPLOY_SSH_KEY 指定 SSH 私钥"
[[ -n "$PUBLIC_ORIGIN" ]] || die "必须通过 --public-origin 或 A4N_DEPLOY_PUBLIC_ORIGIN 指定公网 origin"
case "$PUBLIC_ORIGIN" in
    http://*|https://*) ;;
    *) die "公网 origin 必须使用 http:// 或 https://" ;;
esac
if [[ -z "$REMOTE_DIR" ]]; then
    REMOTE_DIR="/home/$DEPLOY_USER/a4n"
fi

PUBLIC_BASE_PATH="$(normalize_base_path "$PUBLIC_BASE_PATH")"
API_BASE_URL="$(normalize_api_base_url "$API_BASE_URL")"
PUBLIC_ORIGIN="${PUBLIC_ORIGIN%/}"
if [[ -z "$REMOTE_PACKAGE" ]]; then
    REMOTE_PACKAGE="$(dirname "$REMOTE_DIR")/a4n-deploy.tar.gz"
fi
SSH_OPTS=(
    -i "$SSH_KEY"
    -o "StrictHostKeyChecking=${A4N_DEPLOY_STRICT_HOST_KEY_CHECKING:-yes}"
    -o "UserKnownHostsFile=${A4N_DEPLOY_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}"
)

REMOTE="$DEPLOY_USER@$DEPLOY_HOST"
PUBLIC_APP_URL="$PUBLIC_ORIGIN$PUBLIC_BASE_PATH"

check_prerequisites() {
    log_section "前置检查"
    local command
    for command in ssh scp curl tar sha256sum file strings ruby tee flock; do
        require_command "$command"
    done

    load_release_inventory
    require_onnxruntime

    [[ -f "$SSH_KEY" ]] || die "SSH 私钥不存在: $SSH_KEY"
    [[ -r "$SSH_KEY" ]] || die "SSH 私钥不可读: $SSH_KEY"

    if ! $SKIP_BUILD; then
        require_command docker
        require_command node
        require_command npm
    fi

    log_ok "目标: $REMOTE"
    log_ok "远端目录: $REMOTE_DIR"
    log_ok "公网入口: $PUBLIC_APP_URL/"
    log_ok "API Base: $API_BASE_URL"
    log_ok "SSH key: $SSH_KEY"
    log_ok "完整 sidecar: ${#SERVICE_BINARIES[@]} 个"
}

remote_preflight() {
    log_section "远端预检"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$REMOTE_SERVICE" <<'REMOTE'
set -euo pipefail
app="$1"
service="$2"
test -d "$app"
test -d "$app/data"
test -f "$app/.env"
sudo -n true
for command in python3 pgrep tar sha256sum curl; do command -v "$command" >/dev/null; done
df -h "$app"
systemctl is-active "$service"
REMOTE
    log_ok "远端目录、数据目录、.env、sudo 和服务状态正常"
}

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
        docker build -t "$BUILDER_IMAGE" -f "$CACHE_ROOT/Dockerfile.noble-builder" "$CACHE_ROOT"
    else
        log_ok "构建镜像已存在: $BUILDER_IMAGE"
    fi

    docker run --rm "$BUILDER_IMAGE" bash -c \
        'rustc --version; cargo --version; protoc --version; getconf GNU_LIBC_VERSION; test -f /usr/include/google/protobuf/empty.proto'
}

build_backend() {
    log_section "本地容器构建后端"
    ensure_builder_image
    mkdir -p "$CARGO_TARGET_DIR" "$CACHE_ROOT"

    local offline_flag=()
    local docker_cache_mounts=(
        -v "$A4N_ONNXRUNTIME_ROOT:/opt/a4n-onnxruntime:ro"
    )
    if [[ "$CARGO_OFFLINE" == "true" ]]; then
        [[ -d "$HOME/.cargo/registry" ]] || die "离线构建需要 $HOME/.cargo/registry，或使用 --online"
        [[ -d "$HOME/.cargo/git" ]] || die "离线构建需要 $HOME/.cargo/git，或使用 --online"
        offline_flag=(--offline)
        docker_cache_mounts+=(
            -v "$HOME/.cargo/registry:/usr/local/cargo/registry:ro"
            -v "$HOME/.cargo/git:/usr/local/cargo/git:ro"
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
        -e CARGO_HOME=/usr/local/cargo \
        -e RUSTUP_HOME=/usr/local/rustup \
        -e CARGO_BUILD_JOBS="${A4N_DEPLOY_CARGO_JOBS:-2}" \
        -e CARGO_TERM_PROGRESS_WHEN=always \
        -e CARGO_TERM_PROGRESS_WIDTH="${A4N_DEPLOY_CARGO_PROGRESS_WIDTH:-100}" \
        -e CARGO_TERM_COLOR=always \
        -e ORT_LIB_PATH=/opt/a4n-onnxruntime/lib \
        -e ORT_PREFER_DYNAMIC_LINK=1 \
        -e LD_LIBRARY_PATH=/opt/a4n-onnxruntime/lib \
        "${docker_cache_mounts[@]}" \
        -v "$PROJECT_ROOT:/work" \
        -v "$CARGO_TARGET_DIR:/cargo-target" \
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
    if $SKIP_BUILD; then
        log_warn "跳过构建，使用现有产物"
        local binary
        while IFS= read -r binary; do
            [[ -x "$binary" ]] || die "未找到后端二进制: $binary"
        done < <(required_binary_paths)
        [[ -f "$FRONTEND_DIST/index.html" ]] || die "未找到前端 dist: $FRONTEND_DIST"
        return
    fi

    build_frontend
    build_backend
}

prepare_package() {
    log_section "制作部署包"
    rm -rf "$PACKAGE_DIR" "$PACKAGE_PATH"
    mkdir -p "$PACKAGE_DIR/bin/services" "$PACKAGE_DIR/static" "$PACKAGE_DIR/prompts"

    cp "$BINARY_PATH" "$PACKAGE_DIR/bin/a4n-server"
    cp "$CONTROL_BINARY_PATH" "$PACKAGE_DIR/bin/a4n-control"
    cp "$AGENT_WORKER_BINARY_PATH" "$PACKAGE_DIR/bin/a4n-agent-worker"
    cp "$MIGRATION_BINARY_PATH" "$PACKAGE_DIR/bin/a4n-migration"
    local spec binary
    for spec in "${TOOL_SPECS[@]}"; do
        binary="${spec#*:}"
        cp "$CARGO_TARGET_DIR/release/$binary" "$PACKAGE_DIR/bin/$binary"
    done
    for binary in "${SERVICE_BINARIES[@]}"; do
        cp "$CARGO_TARGET_DIR/release/$binary" "$PACKAGE_DIR/bin/services/$binary"
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

    tar -C "$PACKAGE_DIR" -czf "$PACKAGE_PATH" .
    sha256sum "$PACKAGE_PATH" > "$PACKAGE_PATH.sha256"
    PACKAGE_SHA256="$(cut -d' ' -f1 "$PACKAGE_PATH.sha256")"
    PACKAGE_CREATED_AT="$(date --iso-8601=seconds)"
    PACKAGE_SIZE="$(du -h "$PACKAGE_PATH" | cut -f1)"

    log_ok "部署包: $PACKAGE_PATH"
    log_ok "SHA256: $PACKAGE_SHA256"
    log_ok "完整 release manifest: ${#SERVICE_BINARIES[@]} 个 sidecar + ${#TOOL_SPECS[@]} 个工具 + ONNX Runtime"
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

replace_remote_app() {
    log_section "远端预校验、停服、替换与迁移"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$REMOTE_PACKAGE" "$REMOTE_SERVICE" <<'REMOTE'
set -euo pipefail
app="$1"
package_path="$2"
service="$3"
database_url="$(python3 - "$app/.env" <<'PY'
import pathlib
import sys

for raw_line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    line = raw_line.strip()
    if not line or line.startswith("#") or "=" not in line:
        continue
    key, value = line.split("=", 1)
    if key.strip() != "A4N_DATABASE_URL":
        continue
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        value = value[1:-1]
    if not value:
        raise SystemExit("A4N_DATABASE_URL is empty")
    print(value)
    break
else:
    raise SystemExit("A4N_DATABASE_URL is missing")
PY
)"
case "$database_url" in
    postgres://*|postgresql://*) ;;
    *) echo "A4N_DATABASE_URL 必须使用 postgres:// 或 postgresql://" >&2; exit 1 ;;
esac

rm -rf "$app/.deploy-new"
mkdir -p "$app/.deploy-new"
tar -xzf "$package_path" -C "$app/.deploy-new"
test -f "$app/.deploy-new/release-manifest.json"
test -f "$app/.deploy-new/release-manifest.sha256"
test -f "$app/.deploy-new/static/index.html"
test -d "$app/.deploy-new/prompts"
(cd "$app/.deploy-new" && sha256sum -c release-manifest.sha256 >/dev/null)

managed_files="$app/.deploy-new/.managed-files"
python3 - "$app/.deploy-new/release-manifest.json" > "$managed_files" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1]).resolve()
root = manifest_path.parent
manifest = json.loads(manifest_path.read_text())
entries = manifest["binaries"] + manifest["tools"] + manifest["runtime_files"]
paths = [entry["path"] for entry in entries]
if len(paths) != len(set(paths)):
    raise SystemExit("release manifest contains duplicate paths")
for relative in paths:
    candidate = pathlib.PurePosixPath(relative)
    if candidate.is_absolute() or not relative.startswith("bin/") or any(part in {"", ".", ".."} for part in candidate.parts):
        raise SystemExit(f"unsafe release manifest path: {relative}")
    path = (root / pathlib.Path(*candidate.parts)).resolve()
    if root not in path.parents or not path.is_file():
        raise SystemExit(f"missing release manifest file: {relative}")
    print(relative)
PY
mapfile -t managed_files < "$managed_files"
((${#managed_files[@]} > 0))

service_stopped=false
restart_after_failure() {
    local status=$?
    if [[ "$service_stopped" == true && "$status" -ne 0 ]]; then
        sudo systemctl start "$service" || true
    fi
    exit "$status"
}
trap restart_after_failure EXIT

sudo systemctl stop "$service"
if systemctl is-active --quiet "$service"; then
    echo "服务停止失败: $service" >&2
    exit 1
fi
service_stopped=true

ts=$(date +%Y%m%d%H%M%S)
backup="$app/backups/deploy-$ts"

mkdir -p "$backup" "$app/bin"
for relative in "${managed_files[@]}"; do
    case "$relative" in
        bin/services/*) continue ;;
    esac
    if [[ -f "$app/$relative" ]]; then
        mkdir -p "$backup/$(dirname "$relative")"
        cp -a "$app/$relative" "$backup/$relative"
    fi
done
if [[ -d "$app/bin/services" ]]; then
    mkdir -p "$backup/bin"
    cp -a "$app/bin/services" "$backup/bin/services"
fi
for relative in release-manifest.json release-manifest.sha256; do
    if [[ -f "$app/$relative" ]]; then cp -a "$app/$relative" "$backup/$relative"; fi
done
if [[ -d "$app/static" ]]; then cp -a "$app/static" "$backup/static"; fi
if [[ -d "$app/prompts" ]]; then cp -a "$app/prompts" "$backup/prompts"; fi

for relative in "${managed_files[@]}"; do
    case "$relative" in
        bin/services/*) continue ;;
    esac
    destination="$app/$relative"
    mkdir -p "$(dirname "$destination")"
    install -m 755 "$app/.deploy-new/$relative" "$destination.new"
    mv "$destination.new" "$destination"
done

rm -rf "$app/bin/services.new"
cp -a "$app/.deploy-new/bin/services" "$app/bin/services.new"
rm -rf "$app/bin/services"
mv "$app/bin/services.new" "$app/bin/services"

for relative in release-manifest.json release-manifest.sha256; do
    install -m 644 "$app/.deploy-new/$relative" "$app/$relative.new"
    mv "$app/$relative.new" "$app/$relative"
done

rm -rf "$app/static.new" "$app/prompts.new"
cp -a "$app/.deploy-new/static" "$app/static.new"
cp -a "$app/.deploy-new/prompts" "$app/prompts.new"
rm -rf "$app/static"
mv "$app/static.new" "$app/static"
rm -rf "$app/prompts"
mv "$app/prompts.new" "$app/prompts"

A4N_DATABASE_URL="$database_url" "$app/bin/a4n-migration"
rm -rf "$app/.deploy-new"

echo "backup=$backup"
ls -lh "$app/bin/a4n-server" "$app/bin/a4n-control" "$app/bin/a4n-agent-worker" "$app/bin/a4n-migration"
ls -lh "$app/bin/services"
find "$app/static" -maxdepth 2 -type f | wc -l
du -sh "$app/static" "$app/prompts"
trap - EXIT
REMOTE
    log_ok "远端文件替换与数据库迁移完成"
}

update_remote_public_app_env() {
    log_section "更新远端公网入口"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$PUBLIC_APP_URL" <<'REMOTE'
set -euo pipefail
app="$1"
public_app_url="$2"
env_file="$app/.env"
test -f "$env_file"
python3 - "$env_file" "$public_app_url" <<'PY'
import pathlib
import sys

env_path = pathlib.Path(sys.argv[1])
public_app_url = sys.argv[2].strip().rstrip("/")
lines = env_path.read_text().splitlines()
key = "A4N_PUBLIC_APP_URL"
written = False
out = []
for line in lines:
    if "=" in line and not line.lstrip().startswith("#"):
        current_key, _ = line.split("=", 1)
        if current_key == key:
            out.append(f"{key}={public_app_url}")
            written = True
            continue
    out.append(line)

if not written:
    out.append(f"{key}={public_app_url}")

env_path.write_text("\n".join(out) + "\n")
PY
grep '^A4N_PUBLIC_APP_URL=' "$env_file"
REMOTE
    log_ok "远端 A4N_PUBLIC_APP_URL 已更新"
}

restart_remote_service() {
    log_section "重启服务"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_SERVICE" <<'REMOTE'
set -euo pipefail
service="$1"
sudo systemctl restart "$service"
state=activating
for _ in {1..60}; do
    state="$(systemctl is-active "$service" 2>/dev/null || true)"
    [[ "$state" == active || "$state" == failed ]] && break
    sleep 1
done
if [[ "$state" != active ]]; then
    echo "服务启动失败或超时: $service state=$state" >&2
    systemctl status "$service" --no-pager --lines=80 >&2 || true
    journalctl -u "$service" --since "5 minutes ago" --no-pager | tail -n 120 >&2 || true
    exit 1
fi
echo "service_state=$state"
systemctl status "$service" --no-pager --lines=20
if command -v nginx >/dev/null 2>&1; then sudo nginx -t; fi
journalctl -u "$service" --since "5 minutes ago" --no-pager | grep -E "ERROR|panic|failed|Failed|Error" || true
REMOTE
    log_ok "服务已重启"
}

verify_remote_runtime() {
    log_section "远端运行时校验"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$REMOTE_SERVICE" "${SERVICE_BINARIES[@]}" <<'REMOTE'
set -euo pipefail
app="$1"
service="$2"
shift 2
systemctl is-active --quiet "$service"
for binary in "$@"; do
    test -x "$app/bin/services/$binary"
    found=false
    for _ in {1..30}; do
        if pgrep -f -- "^$app/bin/services/$binary( |$)" >/dev/null; then
            found=true
            break
        fi
        sleep 1
    done
    if ! $found; then
        echo "sidecar 未运行: $binary" >&2
        systemctl status "$service" --no-pager --lines=80 >&2 || true
        exit 1
    fi
done
health="$(curl -fsS --max-time 10 http://127.0.0.1:8080/api/health)"
[[ "$health" == "ok" ]] || { echo "内部 health 异常: $health" >&2; exit 1; }
echo "service=$service"
echo "sidecars=$#"
echo "health=$health"
REMOTE
    log_ok "${#SERVICE_BINARIES[@]} 个 sidecar 均在运行，内部 health=ok"
}

http_status() {
    curl -sS -o /dev/null -w '%{http_code}' "$1"
}

smoke_test() {
    if ! $RUN_SMOKE; then
        log_warn "跳过公网 smoke test"
        return
    fi

    log_section "公网 Smoke Test"
    local app_status health temp_media_status asset_path asset_status root_status redirect_status

    app_status="$(http_status "$PUBLIC_APP_URL/")"
    [[ "$app_status" == "200" ]] || die "$PUBLIC_APP_URL/ 预期 200，实际 $app_status"
    log_ok "$PUBLIC_APP_URL/ -> 200"

    health="$(curl -fsS "$PUBLIC_APP_URL/api/health")"
    [[ "$health" == "ok" ]] || die "$PUBLIC_APP_URL/api/health 预期 ok，实际 $health"
    log_ok "$PUBLIC_APP_URL/api/health -> ok"

    temp_media_status="$(http_status "$PUBLIC_APP_URL/api/temp-media/00000000000000000000000000000000.png")"
    [[ "$temp_media_status" == "404" ]] || die "$PUBLIC_APP_URL/api/temp-media/... 预期 404，实际 $temp_media_status"
    log_ok "$PUBLIC_APP_URL/api/temp-media/{uuid}.png -> 404"

    redirect_status="$(http_status "$PUBLIC_APP_URL")"
    [[ "$redirect_status" == "301" || "$redirect_status" == "308" || "$redirect_status" == "200" ]] || die "$PUBLIC_APP_URL 预期 301/308/200，实际 $redirect_status"
    log_ok "$PUBLIC_APP_URL -> $redirect_status"

    asset_path="$(grep -m1 -o "$PUBLIC_BASE_PATH/assets/[^\" ]*\\.js" "$FRONTEND_DIST/index.html")" \
        || die "未从 index.html 找到 JS asset 路径"
    asset_status="$(http_status "$PUBLIC_ORIGIN$asset_path")"
    [[ "$asset_status" == "200" ]] || die "$PUBLIC_ORIGIN$asset_path 预期 200，实际 $asset_status"
    log_ok "$PUBLIC_ORIGIN$asset_path -> 200"

    if [[ "$ROOT_EXPECTED_STATUS" != "any" ]]; then
        root_status="$(http_status "$PUBLIC_ORIGIN/")"
        [[ "$root_status" == "$ROOT_EXPECTED_STATUS" ]] || die "$PUBLIC_ORIGIN/ 预期 $ROOT_EXPECTED_STATUS，实际 $root_status"
        log_ok "$PUBLIC_ORIGIN/ -> $ROOT_EXPECTED_STATUS"
    fi
}

cleanup() {
    log_section "清理临时文件"
    if ! $KEEP_PACKAGE; then
        rm -rf "$PACKAGE_DIR" "$PACKAGE_PATH" "$PACKAGE_PATH.sha256"
        ssh "${SSH_OPTS[@]}" "$REMOTE" "rm -f '$REMOTE_PACKAGE'"
        log_ok "已清理本地部署包和远端上传包"
    else
        log_warn "保留本地部署包: $PACKAGE_PATH"
        ssh "${SSH_OPTS[@]}" "$REMOTE" "rm -f '$REMOTE_PACKAGE'"
    fi
}

main() {
    acquire_deploy_lock
    start_deploy_record
    echo -e "${BOLD}${CYAN}A4N 云端一键部署${NC}"
    echo -e "  目标: ${CYAN}$REMOTE${NC}"
    echo -e "  入口: ${CYAN}$PUBLIC_APP_URL/${NC}"
    echo -e "  后端: ${CYAN}本地 Docker 构建 Host、工具、${#SERVICE_BINARIES[@]} 个 sidecar 和 ONNX Runtime${NC}"

    check_prerequisites
    remote_preflight
    local build_started_epoch="$(date +%s)"
    build_all
    BUILD_ELAPSED_SECONDS=$(( $(date +%s) - build_started_epoch ))
    log_info "构建阶段耗时: $(format_elapsed "$BUILD_ELAPSED_SECONDS")"
    verify_backend_binaries
    prepare_package
    upload_package
    update_remote_public_app_env
    replace_remote_app
    restart_remote_service
    verify_remote_runtime
    smoke_test
    cleanup

    echo ""
    echo -e "${GREEN}${BOLD}部署完成: $PUBLIC_APP_URL/${NC}"
}

main "$@"
