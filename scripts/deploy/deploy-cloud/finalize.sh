http_status() {
    curl -sS -o /dev/null -w '%{http_code}' "$1"
}
smoke_test() {
    if ! $RUN_SMOKE; then
        log_warn "跳过公网 smoke test"
        return
    fi

    log_section "公网 Smoke Test"
    local app_status app_html health temp_media_status asset_path asset_src asset_status root_status redirect_status

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

    app_html="$(curl -fsS "$PUBLIC_APP_URL/")"
    asset_src="$(grep -m1 -oE 'src=\"[^\"]+\.js\"' <<< "$app_html" \
        | sed -n 's/^src=\"\([^\"]*\)\"$/\1/p')"
    [[ -n "$asset_src" ]] || die "线上 index.html 没有 JS asset script src"
    [[ "$asset_src" == "$PUBLIC_BASE_PATH/assets/"* ]] \
        || die "线上 index.html 的 JS asset 路径不匹配 $PUBLIC_BASE_PATH/assets: $asset_src"
    asset_path="$asset_src"
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
    if [[ "$DEPLOY_MODE" == noop ]]; then
        log_ok "无增量部署包需要清理"
        return
    fi
    if ! $KEEP_PACKAGE; then
        rm -rf "$PACKAGE_DIR" "$DELTA_DIR" "$PACKAGE_PATH" "$PACKAGE_PATH.sha256"
        ssh "${SSH_OPTS[@]}" "$REMOTE" "rm -f '$REMOTE_PACKAGE'"
        log_ok "已清理本地部署包和远端上传包"
    else
        log_warn "保留本地部署包: $PACKAGE_PATH"
        ssh "${SSH_OPTS[@]}" "$REMOTE" "rm -f '$REMOTE_PACKAGE'"
    fi
}

build_package_and_upload() {
    local build_started_epoch build_elapsed
    build_started_epoch="$(date +%s)"
    build_all
    build_elapsed=$(( $(date +%s) - build_started_epoch ))
    BUILD_ELAPSED_SECONDS=$(( ${BUILD_ELAPSED_SECONDS:-0} + build_elapsed ))
    log_info "构建阶段耗时: $(format_elapsed "$build_elapsed")"
    [[ "$DEPLOY_MODE" == noop ]] && return
    verify_backend_binaries
    prepare_package
    upload_package
}
