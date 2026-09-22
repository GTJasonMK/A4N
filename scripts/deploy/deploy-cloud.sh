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
#   ./scripts/deploy/deploy-cloud.sh --full            # 强制全量构建、上传和重启
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
#   - 默认按部署计划上传变化组件；全量模式上传 Host、工具、sidecar、ONNX Runtime、static 和 prompts，不覆盖远端 .env 与 data。
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
CACHE_ROOT="$CARGO_TARGET_DIR/deploy-cloud"
BUILD_TARGET_DIR="$CACHE_ROOT/target"
BUILD_CARGO_HOME="$CACHE_ROOT/cargo-home"
BINARY_PATH="$BUILD_TARGET_DIR/release/a4n-server"
CONTROL_BINARY_PATH="$BUILD_TARGET_DIR/release/a4n-control"
AGENT_WORKER_BINARY_PATH="$BUILD_TARGET_DIR/release/a4n-agent-worker"
MIGRATION_BINARY_PATH="$BUILD_TARGET_DIR/release/a4n-migration"
HOST_PACKAGE="a4n-server"
TARGET_PLATFORM="linux-x86_64"
RELEASE_MANIFEST_TOOL="$PROJECT_ROOT/scripts/release_manifest.rb"
DEPLOY_PLAN_TOOL="$PROJECT_ROOT/scripts/deploy/deploy-plan.rb"
SERVICE_BINARIES=()
TOOL_SPECS=()
BUILD_PACKAGES=()
PLAN_ARTIFACT_PATHS=()
PLAN_PAYLOADS=()

CARGO_OFFLINE="${A4N_DEPLOY_CARGO_OFFLINE:-true}"
SKIP_BUILD=false
FORCE_FULL=false
REBUILD_BUILDER=false
RUN_SMOKE=true
KEEP_PACKAGE=false

PACKAGE_DIR="$CACHE_ROOT/package"
DELTA_DIR="$CACHE_ROOT/delta-package"
PACKAGE_PATH="$CACHE_ROOT/a4n-deploy.tar.gz"
DELTA_MANIFEST_PATH="$DELTA_DIR/delta-manifest.json"
PACKAGE_SHA256=""
PACKAGE_CREATED_AT=""
PACKAGE_SIZE=""
RELEASE_ID=""
DEPLOY_RECORD_DIR="${A4N_DEPLOY_RECORD_DIR:-$PROJECT_ROOT/deploy-records}"
DEPLOY_RECORD_PATH=""
DEPLOY_RECORD_JSON_PATH=""
DEPLOY_PLAN_PATH="$CACHE_ROOT/deploy-plan.json"
DEPLOY_STATE_JSON_PATH="$CACHE_ROOT/deploy-state.json"
DEPLOY_ID=""
REMOTE_STATE_PATH=""
REMOTE_STATE_CACHE="$CACHE_ROOT/remote-state.json"
DEPLOY_MODE="full"
DEPLOY_FALLBACK_REASON=""
DEPLOY_STARTED_AT="$(date --iso-8601=seconds)"
DEPLOY_STARTED_EPOCH="$(date +%s)"
BUILD_ELAPSED_SECONDS=""
DEPLOY_LOCK_FILE="${A4N_DEPLOY_LOCK_FILE:-$CARGO_TARGET_DIR/deploy-cloud.lock}"
DEPLOY_LOCK_FD=""
DOCKER_BUILD_CIDFILE="$CACHE_ROOT/build-container.cid"
DOCKER_BUILD_PID=""


# shellcheck source=scripts/deploy/deploy-cloud/foundation.sh
source "$SCRIPT_DIR/deploy-cloud/foundation.sh"
# shellcheck source=scripts/deploy/deploy-cloud/preflight.sh
source "$SCRIPT_DIR/deploy-cloud/preflight.sh"
# shellcheck source=scripts/deploy/deploy-cloud/build.sh
source "$SCRIPT_DIR/deploy-cloud/build.sh"
# shellcheck source=scripts/deploy/deploy-cloud/remote.sh
source "$SCRIPT_DIR/deploy-cloud/remote.sh"
# shellcheck source=scripts/deploy/deploy-cloud/finalize.sh
source "$SCRIPT_DIR/deploy-cloud/finalize.sh"

main() {
    acquire_deploy_lock
    trap finish_deploy_record EXIT
    trap 'handle_deploy_signal 130' INT
    trap 'handle_deploy_signal 143' TERM
    trap 'handle_deploy_signal 129' HUP
    start_deploy_record
    echo -e "${BOLD}${CYAN}A4N 云端一键部署${NC}"
    echo -e "  目标: ${CYAN}$REMOTE${NC}"
    echo -e "  入口: ${CYAN}$PUBLIC_APP_URL/${NC}"
    echo -e "  后端: ${CYAN}本地 Docker 按部署计划构建受影响组件${NC}"

    check_prerequisites
    remote_preflight
    fetch_remote_state
    prepare_deploy_plan
    build_package_and_upload
    if [[ "$DEPLOY_MODE" == noop ]]; then
        verify_remote_runtime
        smoke_test
        cleanup
        echo ""
        echo -e "${GREEN}${BOLD}无需部署，当前 release 已是最新: $PUBLIC_APP_URL/${NC}"
        return
    fi

    if [[ "$DEPLOY_MODE" == delta ]]; then
        if replace_remote_delta; then
            :
        else
            delta_status=$?
            if [[ "$delta_status" -ne 42 ]]; then
                return "$delta_status"
            fi
            log_warn "远端基线已变化，重新计算并执行全量部署"
            FORCE_FULL=true
            prepare_deploy_plan
            build_package_and_upload
            update_remote_public_app_env
            replace_remote_app
            restart_remote_service
        fi
    else
        update_remote_public_app_env
        replace_remote_app
        restart_remote_service
    fi
    verify_remote_runtime
    smoke_test
    prepare_active_state
    sync_remote_state
    cleanup

    echo ""
    echo -e "${GREEN}${BOLD}部署完成: $PUBLIC_APP_URL/${NC}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
