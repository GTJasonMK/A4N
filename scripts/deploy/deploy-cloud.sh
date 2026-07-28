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
#   ./scripts/deploy/deploy-cloud.sh --skip-build      # 使用已有 target-noble/release/a4n-server 和 frontend/dist
#   ./scripts/deploy/deploy-cloud.sh --rebuild-builder # 重建本地 Ubuntu 24 构建镜像
#   ./scripts/deploy/deploy-cloud.sh --no-smoke        # 跳过公网 smoke test
#   ./scripts/deploy/deploy-cloud.sh --help
#
# 约束:
#   - 后端只在本地 Ubuntu 24 / glibc 2.39 Docker 容器中构建，不在 2G 云服务器上构建。
#   - 只上传 bin/a4n-server、static 和 prompts，不覆盖远端 .env 与 data。
#   - 部署 Host、SSH key 和公网 origin 必须显式提供，根路径默认期望 404。
#
# 常用环境变量覆盖:
#   A4N_DEPLOY_HOST, A4N_DEPLOY_USER, A4N_DEPLOY_SSH_KEY
#   A4N_DEPLOY_REMOTE_DIR, A4N_DEPLOY_SERVICE, A4N_DEPLOY_BASE_PATH
#   A4N_DEPLOY_API_BASE_URL, A4N_DEPLOY_PUBLIC_ORIGIN
#   A4N_DEPLOY_CARGO_OFFLINE=false
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

FRONTEND_DIR="$PROJECT_ROOT/frontend"
FRONTEND_DIST="$FRONTEND_DIR/dist"
PROMPTS_DIR="$PROJECT_ROOT/prompts"

DEPLOY_HOST="${A4N_DEPLOY_HOST:-}"
DEPLOY_USER="${A4N_DEPLOY_USER:-ubuntu}"
SSH_KEY="${A4N_DEPLOY_SSH_KEY:-}"
REMOTE_DIR="${A4N_DEPLOY_REMOTE_DIR:-}"
REMOTE_PACKAGE="${A4N_DEPLOY_REMOTE_PACKAGE:-}"
REMOTE_SERVICE="${A4N_DEPLOY_SERVICE:-a4n.service}"
REMOTE_NGINX_CONF="${A4N_DEPLOY_NGINX_CONF:-/etc/nginx/conf.d/a4n.conf}"

PUBLIC_BASE_PATH="${A4N_DEPLOY_BASE_PATH:-/a4n}"
PUBLIC_ORIGIN="${A4N_DEPLOY_PUBLIC_ORIGIN:-}"
API_BASE_URL="${A4N_DEPLOY_API_BASE_URL:-$PUBLIC_BASE_PATH/api}"
TEMP_MEDIA_PUBLIC_BASE_URL="${A4N_DEPLOY_TEMP_MEDIA_PUBLIC_BASE_URL:-}"
ROOT_EXPECTED_STATUS="${A4N_DEPLOY_ROOT_EXPECTED_STATUS:-404}"

BUILDER_IMAGE="${A4N_DEPLOY_BUILDER_IMAGE:-a4n-rust-noble:1.95}"
RUST_BASE_IMAGE="${A4N_DEPLOY_RUST_BASE:-docker.m.daocloud.io/library/rust:1.95-bookworm}"
UBUNTU_BASE_IMAGE="${A4N_DEPLOY_UBUNTU_BASE:-docker.m.daocloud.io/library/ubuntu:24.04}"
TARGET_DIR_REL="${A4N_DEPLOY_TARGET_DIR:-target-noble}"
BINARY_PATH="$PROJECT_ROOT/$TARGET_DIR_REL/release/a4n-server"

CARGO_OFFLINE="${A4N_DEPLOY_CARGO_OFFLINE:-true}"
SKIP_BUILD=false
REBUILD_BUILDER=false
RUN_SMOKE=true
KEEP_PACKAGE=false

CACHE_ROOT="${A4N_DEPLOY_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/a4n-deploy-cloud}"
PACKAGE_DIR="$CACHE_ROOT/package"
PACKAGE_PATH="$CACHE_ROOT/a4n-deploy.tar.gz"

usage() {
    print_script_usage "$0" 23
}

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
            SSH_OPTS[1]="$SSH_KEY"
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
            API_BASE_URL="${A4N_DEPLOY_API_BASE_URL:-$PUBLIC_BASE_PATH/api}"
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

check_command() {
    command -v "$1" >/dev/null 2>&1 || die "未找到命令: $1"
}

check_prerequisites() {
    log_section "前置检查"
    check_command ssh
    check_command scp
    check_command curl
    check_command tar
    check_command sha256sum
    check_command file
    check_command strings

    [[ -f "$SSH_KEY" ]] || die "SSH 私钥不存在: $SSH_KEY"
    [[ -r "$SSH_KEY" ]] || die "SSH 私钥不可读: $SSH_KEY"

    if ! $SKIP_BUILD; then
        check_command docker
        check_command node
        check_command npm
    fi

    log_ok "目标: $REMOTE"
    log_ok "远端目录: $REMOTE_DIR"
    log_ok "公网入口: $PUBLIC_APP_URL/"
    log_ok "API Base: $API_BASE_URL"
    log_ok "SSH key: $SSH_KEY"
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
df -h "$app"
systemctl is-active "$service"
REMOTE
    log_ok "远端目录、数据目录、.env、sudo 和服务状态正常"
}

ensure_frontend_deps() {
    if [[ ! -d "$FRONTEND_DIR/node_modules" ]]; then
        log_warn "前端依赖未安装，执行 npm install"
        (cd "$FRONTEND_DIR" && npm install --silent)
    fi
}

build_frontend() {
    log_section "构建前端"
    ensure_frontend_deps
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

    local offline_flag=()
    local docker_cache_mounts=()
    if [[ "$CARGO_OFFLINE" == "true" ]]; then
        [[ -d "$HOME/.cargo/registry" ]] || die "离线构建需要 $HOME/.cargo/registry，或使用 --online"
        [[ -d "$HOME/.cargo/git" ]] || die "离线构建需要 $HOME/.cargo/git，或使用 --online"
        offline_flag=(--offline)
        docker_cache_mounts=(
            -v "$HOME/.cargo/registry:/usr/local/cargo/registry:ro"
            -v "$HOME/.cargo/git:/usr/local/cargo/git:ro"
        )
        log_info "cargo 使用离线构建"
    else
        log_info "cargo 允许联网构建"
    fi

    docker run --rm \
        -e CARGO_HOME=/usr/local/cargo \
        -e RUSTUP_HOME=/usr/local/rustup \
        -e CARGO_PROFILE_RELEASE_LTO=false \
        -e CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
        -e CARGO_BUILD_JOBS="${A4N_DEPLOY_CARGO_JOBS:-2}" \
        "${docker_cache_mounts[@]}" \
        -v "$PROJECT_ROOT:/work" \
        -w /work \
        "$BUILDER_IMAGE" \
        bash -c "set -euo pipefail; rustc --version; getconf GNU_LIBC_VERSION; protoc --version; cargo build --release -p a4n-server --target-dir /work/$TARGET_DIR_REL --locked ${offline_flag[*]}"

    [[ -x "$BINARY_PATH" ]] || die "后端构建后未找到二进制: $BINARY_PATH"
    log_ok "后端二进制: $BINARY_PATH"
}

verify_backend_binary() {
    log_section "校验后端二进制"
    [[ -x "$BINARY_PATH" ]] || die "后端二进制不存在: $BINARY_PATH"
    file "$BINARY_PATH"

    local max_glibc
    max_glibc=$(strings "$BINARY_PATH" | grep -E '^GLIBC_[0-9]' | sort -V | tail -1 || true)
    [[ -n "$max_glibc" ]] || die "无法识别二进制 GLIBC 依赖"
    log_ok "最高 GLIBC 依赖: $max_glibc"
    if [[ "$(printf '%s\nGLIBC_2.39\n' "$max_glibc" | sort -V | tail -1)" != "GLIBC_2.39" ]]; then
        die "二进制依赖 $max_glibc，高于线上 Ubuntu 24.04 的 GLIBC_2.39"
    fi
}

build_all() {
    if $SKIP_BUILD; then
        log_warn "跳过构建，使用现有产物"
        [[ -x "$BINARY_PATH" ]] || die "未找到后端二进制: $BINARY_PATH"
        [[ -f "$FRONTEND_DIST/index.html" ]] || die "未找到前端 dist: $FRONTEND_DIST"
        return
    fi

    build_frontend
    build_backend
}

prepare_package() {
    log_section "制作部署包"
    rm -rf "$PACKAGE_DIR" "$PACKAGE_PATH"
    mkdir -p "$PACKAGE_DIR/bin" "$PACKAGE_DIR/static" "$PACKAGE_DIR/prompts"

    cp "$BINARY_PATH" "$PACKAGE_DIR/bin/a4n-server"
    cp -a "$FRONTEND_DIST/." "$PACKAGE_DIR/static/"
    cp -a "$PROMPTS_DIR/." "$PACKAGE_DIR/prompts/"
    chmod 755 "$PACKAGE_DIR/bin/a4n-server"

    tar -C "$PACKAGE_DIR" -czf "$PACKAGE_PATH" .
    sha256sum "$PACKAGE_PATH" > "$PACKAGE_PATH.sha256"

    log_ok "部署包: $PACKAGE_PATH"
    log_ok "SHA256: $(cut -d' ' -f1 "$PACKAGE_PATH.sha256")"
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
    log_section "远端备份与替换"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$REMOTE_PACKAGE" "$REMOTE_NGINX_CONF" <<'REMOTE'
set -euo pipefail
app="$1"
package_path="$2"
nginx_conf="$3"
ts=$(date +%Y%m%d%H%M%S)
backup="$app/backups/deploy-$ts"

mkdir -p "$backup" "$app/bin"
if [[ -f "$app/bin/a4n-server" ]]; then cp -a "$app/bin/a4n-server" "$backup/a4n-server"; fi
if [[ -d "$app/static" ]]; then cp -a "$app/static" "$backup/static"; fi
if [[ -d "$app/prompts" ]]; then cp -a "$app/prompts" "$backup/prompts"; fi
if [[ -n "$nginx_conf" && -f "$nginx_conf" ]]; then sudo cp -a "$nginx_conf" "$backup/$(basename "$nginx_conf")" || true; fi

rm -rf "$app/.deploy-new"
mkdir -p "$app/.deploy-new"
tar -xzf "$package_path" -C "$app/.deploy-new"

test -x "$app/.deploy-new/bin/a4n-server"
test -f "$app/.deploy-new/static/index.html"
test -d "$app/.deploy-new/prompts"

install -m 755 "$app/.deploy-new/bin/a4n-server" "$app/bin/a4n-server.new"
mv "$app/bin/a4n-server.new" "$app/bin/a4n-server"

rm -rf "$app/static.new" "$app/prompts.new"
cp -a "$app/.deploy-new/static" "$app/static.new"
cp -a "$app/.deploy-new/prompts" "$app/prompts.new"
rm -rf "$app/static"
mv "$app/static.new" "$app/static"
rm -rf "$app/prompts"
mv "$app/prompts.new" "$app/prompts"
rm -rf "$app/.deploy-new"

echo "backup=$backup"
ls -lh "$app/bin/a4n-server"
find "$app/static" -maxdepth 2 -type f | wc -l
du -sh "$app/static" "$app/prompts"
REMOTE
    log_ok "远端文件替换完成"
}

ensure_remote_public_app_env() {
    log_section "更新远端公网与临时媒体入口"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$PUBLIC_APP_URL" "$TEMP_MEDIA_PUBLIC_BASE_URL" <<'REMOTE'
set -euo pipefail
app="$1"
public_app_url="$2"
explicit_temp_media_public_base_url="${3:-}"
env_file="$app/.env"
test -f "$env_file"
python3 - "$env_file" "$public_app_url" "$explicit_temp_media_public_base_url" <<'PY'
import pathlib
import secrets
import sys

env_path = pathlib.Path(sys.argv[1])
public_app_url = sys.argv[2].strip().rstrip("/")
explicit_temp_media_public_base_url = sys.argv[3].strip().rstrip("/")
lines = env_path.read_text().splitlines()
values = {}
for line in lines:
    if "=" in line and not line.lstrip().startswith("#"):
        key, value = line.split("=", 1)
        values[key] = value.strip()

temp_media_public_base_url = (
    explicit_temp_media_public_base_url
    or values.get("A4N_TEMP_MEDIA_PUBLIC_BASE_URL", "").strip().rstrip("/")
    or f"{public_app_url}/api/temp-media"
)

required = {
    "A4N_PUBLIC_APP_URL": public_app_url,
    "A4N_TEMP_MEDIA_PUBLIC_BASE_URL": temp_media_public_base_url,
    "A4N_TEMP_MEDIA_TTL_MINUTES": values.get("A4N_TEMP_MEDIA_TTL_MINUTES", "360"),
    "A4N_TEMP_MEDIA_MAX_BYTES": values.get("A4N_TEMP_MEDIA_MAX_BYTES", str(20 * 1024 * 1024)),
}
required["A4N_TEMP_MEDIA_UPLOAD_TOKEN"] = values.get(
    "A4N_TEMP_MEDIA_UPLOAD_TOKEN",
    secrets.token_urlsafe(48),
)

written = set()
out = []
for line in lines:
    if "=" in line and not line.lstrip().startswith("#"):
        key, _ = line.split("=", 1)
        if key in required:
            out.append(f"{key}={required[key]}")
            written.add(key)
            continue
    out.append(line)

for key, value in required.items():
    if key not in written:
        out.append(f"{key}={value}")

env_path.write_text("\n".join(out) + "\n")
PY
grep '^A4N_PUBLIC_APP_URL=' "$env_file"
grep '^A4N_TEMP_MEDIA_PUBLIC_BASE_URL=' "$env_file"
grep '^A4N_TEMP_MEDIA_TTL_MINUTES=' "$env_file"
if grep -q '^A4N_TEMP_MEDIA_UPLOAD_TOKEN=' "$env_file"; then
    echo "A4N_TEMP_MEDIA_UPLOAD_TOKEN=<configured>"
fi
REMOTE
    log_ok "远端 A4N_PUBLIC_APP_URL 与临时媒体入口已更新"
}

restart_remote_service() {
    log_section "重启服务"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_SERVICE" <<'REMOTE'
set -euo pipefail
service="$1"
sudo systemctl restart "$service"
sleep 3
systemctl is-active "$service"
systemctl status "$service" --no-pager --lines=20
if command -v nginx >/dev/null 2>&1; then sudo nginx -t; fi
journalctl -u "$service" --since "5 minutes ago" --no-pager | grep -E "ERROR|panic|failed|Failed|Error" || true
REMOTE
    log_ok "服务已重启"
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

    asset_path="$(grep -o "$PUBLIC_BASE_PATH/assets/[^\" ]*\\.js" "$FRONTEND_DIST/index.html" | head -1 || true)"
    if [[ -n "$asset_path" ]]; then
        asset_status="$(http_status "$PUBLIC_ORIGIN$asset_path")"
        [[ "$asset_status" == "200" ]] || die "$PUBLIC_ORIGIN$asset_path 预期 200，实际 $asset_status"
        log_ok "$PUBLIC_ORIGIN$asset_path -> 200"
    else
        log_warn "未从 index.html 找到 JS asset 路径，跳过静态资源 smoke"
    fi

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
        ssh "${SSH_OPTS[@]}" "$REMOTE" "rm -f '$REMOTE_PACKAGE'" || true
        log_ok "已清理本地部署包和远端上传包"
    else
        log_warn "保留本地部署包: $PACKAGE_PATH"
        ssh "${SSH_OPTS[@]}" "$REMOTE" "rm -f '$REMOTE_PACKAGE'" || true
    fi
}

main() {
    echo -e "${BOLD}${CYAN}A4N 云端一键部署${NC}"
    echo -e "  目标: ${CYAN}$REMOTE${NC}"
    echo -e "  入口: ${CYAN}$PUBLIC_APP_URL/${NC}"
    echo -e "  后端: ${CYAN}本地 Docker 构建 a4n-server${NC}"

    check_prerequisites
    remote_preflight
    build_all
    verify_backend_binary
    prepare_package
    upload_package
    replace_remote_app
    ensure_remote_public_app_env
    restart_remote_service
    smoke_test
    cleanup

    echo ""
    echo -e "${GREEN}${BOLD}部署完成: $PUBLIC_APP_URL/${NC}"
}

main "$@"
