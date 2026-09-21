check_prerequisites() {
    log_section "前置检查"
    local command
    for command in ssh scp curl tar sha256sum file strings ruby tee flock cargo; do
        require_command "$command"
    done
    [[ -r "$DEPLOY_PLAN_TOOL" ]] || die "缺少部署计划工具: $DEPLOY_PLAN_TOOL"

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

fetch_remote_state() {
    log_section "读取远端部署状态"
    mkdir -p "$CACHE_ROOT"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" <<'REMOTE' > "$REMOTE_STATE_CACHE"
set -euo pipefail
app="$1"
state="$app/.deploy-state/current.json"
if [[ -r "$state" ]]; then
    cat "$state"
else
    printf '{}\n'
fi
REMOTE
    ruby -rjson -e 'JSON.parse(File.read(ARGV.fetch(0)))' "$REMOTE_STATE_CACHE" \
        || die "远端部署状态不是有效 JSON: $REMOTE_STATE_CACHE"
    if [[ "$(ruby -rjson -e 'puts JSON.parse(File.read(ARGV.fetch(0))).fetch("release_id", "")' "$REMOTE_STATE_CACHE")" == "" ]]; then
        log_warn "远端尚未建立部署基线，首次部署将执行全量模式"
    else
        log_ok "已读取远端 release 基线"
    fi
}

prepare_deploy_plan() {
    log_section "分析增量部署范围"
    mkdir -p "$CACHE_ROOT"
    local metadata_path="$CACHE_ROOT/cargo-metadata.json"
    local -a metadata_args=(metadata --format-version 1 --locked --manifest-path "$PROJECT_ROOT/Cargo.toml")
    [[ "$CARGO_OFFLINE" == true ]] && metadata_args+=(--offline)
    cargo "${metadata_args[@]}" > "$metadata_path"

    local requested_mode=auto
    $FORCE_FULL && requested_mode=full
    ruby "$DEPLOY_PLAN_TOOL" plan \
        --root "$PROJECT_ROOT" \
        --metadata "$metadata_path" \
        --state "$REMOTE_STATE_CACHE" \
        --output "$DEPLOY_PLAN_PATH" \
        --mode "$requested_mode" \
        --target "$TARGET_PLATFORM" \
        --builder "$BUILDER_IMAGE" \
        --cargo-offline "$CARGO_OFFLINE" \
        --public-app-url "$PUBLIC_APP_URL" \
        --public-base-path "$PUBLIC_BASE_PATH" \
        --api-base-url "$API_BASE_URL"

    DEPLOY_MODE="$(plan_emit mode)"
    DEPLOY_FALLBACK_REASON="$(plan_emit fallback-reason)"
    mapfile -t BUILD_PACKAGES < <(plan_emit build-packages)
    mapfile -t PLAN_ARTIFACT_PATHS < <(plan_emit artifact-paths)
    mapfile -t PLAN_PAYLOADS < <(plan_emit payloads)

    if [[ "$DEPLOY_MODE" == delta ]]; then
        local reusable=true binary
        while IFS= read -r binary; do
            if [[ ! -x "$binary" ]]; then
                reusable=false
                break
            fi
        done < <(required_binary_paths)
        [[ -f "$FRONTEND_DIST/index.html" ]] || reusable=false
        if ! $reusable; then
            FORCE_FULL=true
            log_warn "本地缺少未变化的基线产物，回退全量构建"
            prepare_deploy_plan
            return
        fi
    fi

    if [[ -n "$DEPLOY_FALLBACK_REASON" ]]; then
        log_warn "部署范围: $DEPLOY_MODE，原因: $DEPLOY_FALLBACK_REASON"
    else
        log_ok "部署范围: $DEPLOY_MODE；构建 package: ${BUILD_PACKAGES[*]:-无}"
    fi
}
