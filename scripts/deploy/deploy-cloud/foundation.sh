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
        printf '%s/release/%s\n' "$BUILD_TARGET_DIR" "$binary"
    done
    for binary in "${SERVICE_BINARIES[@]}"; do
        printf '%s/release/%s\n' "$BUILD_TARGET_DIR" "$binary"
    done
}

format_elapsed() {
    local total_seconds="$1"
    printf '%02dh %02dm %02ds' \
        "$((total_seconds / 3600))" \
        "$((total_seconds % 3600 / 60))" \
        "$((total_seconds % 60))"
}

plan_emit() {
    local field="$1"
    ruby "$DEPLOY_PLAN_TOOL" emit --plan "$DEPLOY_PLAN_PATH" --output "$field"
}

write_deploy_record_json() {
    local status="$1"
    local exit_status="$2"
    local finished_at elapsed_seconds
    local -a record_args
    finished_at="$(date --iso-8601=seconds)"
    elapsed_seconds=$(( $(date +%s) - DEPLOY_STARTED_EPOCH ))
    [[ -n "$DEPLOY_RECORD_JSON_PATH" ]] || return 0
    record_args=(
        record
        --plan "$DEPLOY_PLAN_PATH" \
        --manifest "$PACKAGE_DIR/release-manifest.json"
        --output "$DEPLOY_RECORD_JSON_PATH"
        --status "$status"
        --exit-status "$exit_status"
        --deployment-id "$DEPLOY_ID"
        --started-at "$DEPLOY_STARTED_AT"
        --finished-at "$finished_at"
        --elapsed-seconds "$elapsed_seconds"
        --remote "$REMOTE" \
        --remote-dir "$REMOTE_DIR" \
        --service "$REMOTE_SERVICE" \
        --public-app-url "$PUBLIC_APP_URL"
    )
    [[ -n "$RELEASE_ID" ]] && record_args+=(--release-id "$RELEASE_ID")
    [[ -n "$BUILD_ELAPSED_SECONDS" ]] && record_args+=(--build-elapsed-seconds "$BUILD_ELAPSED_SECONDS")
    [[ -n "$PACKAGE_SHA256" ]] && record_args+=(--package-sha256 "$PACKAGE_SHA256")
    [[ -n "$PACKAGE_SIZE" ]] && record_args+=(--package-size "$PACKAGE_SIZE")
    [[ -n "$PACKAGE_CREATED_AT" ]] && record_args+=(--package-created-at "$PACKAGE_CREATED_AT")
    ruby "$DEPLOY_PLAN_TOOL" "${record_args[@]}" \
        || log_warn "无法生成机器可读部署记录: $DEPLOY_RECORD_JSON_PATH"
}

start_deploy_record() {
    command -v tee >/dev/null 2>&1 || die "未找到命令: tee"
    DEPLOY_STARTED_AT="$(date --iso-8601=seconds)"
    DEPLOY_STARTED_EPOCH="$(date +%s)"
    mkdir -p "$DEPLOY_RECORD_DIR"
    DEPLOY_ID="$(date +%Y%m%d%H%M%S)-$$"
    DEPLOY_RECORD_PATH="$DEPLOY_RECORD_DIR/deploy-$DEPLOY_ID.log"
    DEPLOY_RECORD_JSON_PATH="$DEPLOY_RECORD_DIR/deploy-$DEPLOY_ID.json"
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
        write_deploy_record_json "$([[ "$status" -eq 0 ]] && echo success || echo failed)" "$status"
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
        --full)
            FORCE_FULL=true
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
REMOTE_STATE_PATH="$REMOTE_DIR/.deploy-state/current.json"
SSH_OPTS=(
    -i "$SSH_KEY"
    -o "StrictHostKeyChecking=${A4N_DEPLOY_STRICT_HOST_KEY_CHECKING:-yes}"
    -o "UserKnownHostsFile=${A4N_DEPLOY_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}"
)

REMOTE="$DEPLOY_USER@$DEPLOY_HOST"
PUBLIC_APP_URL="$PUBLIC_ORIGIN$PUBLIC_BASE_PATH"
