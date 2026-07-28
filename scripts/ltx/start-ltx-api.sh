#!/usr/bin/env bash
#=============================================================================
# Start local LTX 2.3 video API bridge for A4N.
#
# Usage:
#   ./scripts/ltx/start-ltx-api.sh
#   ./scripts/ltx/start-ltx-api.sh --no-remote-start
#   ./scripts/ltx/start-ltx-api.sh --no-tunnel
#   ./scripts/ltx/start-ltx-api.sh --no-model-check
#   ./scripts/ltx/start-ltx-api.sh --help
#
# A4N video config:
#   Provider: OpenAI compatible video
#   Base URL: http://127.0.0.1:8787/v1
#   Model: ltx-2.3
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

load_dotenv

APP_DATA_DIR="${A4N_DATA_DIR:-$PROJECT_ROOT/data}"
APP_TMP_DIR="${A4N_TEMP_DIR:-$APP_DATA_DIR/tmp}"
mkdir -p "$APP_TMP_DIR"

LTX_SSH_HOST="${LTX_SSH_HOST:-}"
LTX_SSH_PORT="${LTX_SSH_PORT:-22}"
LTX_SSH_USER="${LTX_SSH_USER:-root}"
LTX_LOCAL_COMFY_PORT="${LTX_LOCAL_COMFY_PORT:-6006}"
LTX_REMOTE_COMFY_HOST="${LTX_REMOTE_COMFY_HOST:-127.0.0.1}"
LTX_REMOTE_COMFY_PORT="${LTX_REMOTE_COMFY_PORT:-6006}"
LTX_REMOTE_COMFY_ROOT="${LTX_REMOTE_COMFY_ROOT:-/root/ComfyUI}"
LTX_REMOTE_PYTHON_BIN="${LTX_REMOTE_PYTHON_BIN:-/root/miniconda3/bin/python}"
LTX_REMOTE_COMFY_LOG="${LTX_REMOTE_COMFY_LOG:-/tmp/comfyui-start.log}"
LTX_REMOTE_HF_ENDPOINT="${LTX_REMOTE_HF_ENDPOINT:-https://hf-mirror.com}"
LTX_REMOTE_HF_HOME="${LTX_REMOTE_HF_HOME:-/root/autodl-tmp/huggingface}"
LTX_REMOTE_HF_HUB_CACHE="${LTX_REMOTE_HF_HUB_CACHE:-${LTX_REMOTE_HF_HOME%/}/hub}"
LTX_REMOTE_HF_XET_CACHE="${LTX_REMOTE_HF_XET_CACHE:-${LTX_REMOTE_HF_HOME%/}/xet}"
LTX_REMOTE_HF_HUB_DISABLE_XET="${LTX_REMOTE_HF_HUB_DISABLE_XET:-0}"
LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT="${LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT:-60}"
LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS="${LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS:-8}"
LTX_REMOTE_HTTP_PROXY="${LTX_REMOTE_HTTP_PROXY:-}"
LTX_REMOTE_HTTPS_PROXY="${LTX_REMOTE_HTTPS_PROXY:-}"
LTX_REMOTE_ALL_PROXY="${LTX_REMOTE_ALL_PROXY:-}"
LTX_REMOTE_NO_PROXY="${LTX_REMOTE_NO_PROXY:-127.0.0.1,localhost}"
LTX_BRIDGE_HOST="${LTX_BRIDGE_HOST:-127.0.0.1}"
LTX_BRIDGE_PORT="${LTX_BRIDGE_PORT:-8787}"
LTX_BRIDGE_API_KEY="${LTX_BRIDGE_API_KEY:-ltx-local}"
LTX_BRIDGE_CONFIG="${LTX_BRIDGE_CONFIG:-$APP_DATA_DIR/ltx-bridge.config.json}"
LTX_COMFY_BASE_URL="${LTX_COMFY_BASE_URL:-http://127.0.0.1:${LTX_LOCAL_COMFY_PORT}}"
LTX_CHECKPOINT_NAME="${LTX_CHECKPOINT_NAME:-ltx-2.3-22b-dev-fp8.safetensors}"
LTX_LORA_NAME="${LTX_LORA_NAME:-Ltx/ltx-2.3-22b-distilled-lora-384-1.1.safetensors}"
LTX_UPSCALER_NAME="${LTX_UPSCALER_NAME:-ltx-2.3-spatial-upscaler-x2-1.1.safetensors}"
LTX_TEXT_ENCODER_NAME="${LTX_TEXT_ENCODER_NAME:-gemma_3_12B_it_fp4_mixed.safetensors}"
ZIMAGE_MODEL_NAME="${ZIMAGE_MODEL_NAME:-nunchaku/svdq-fp4_r128-z-image-turbo.safetensors}"
ZIMAGE_TEXT_ENCODER_NAME="${ZIMAGE_TEXT_ENCODER_NAME:-qwen_3_4b.safetensors}"
ZIMAGE_VAE_NAME="${ZIMAGE_VAE_NAME:-ae.safetensors}"
ZIMAGE_CONTROLNET_PATCH_NAME="${ZIMAGE_CONTROLNET_PATCH_NAME:-Z-Image-Turbo-Fun-Controlnet-Union.safetensors}"
LTX_SKIP_REMOTE_START="${LTX_SKIP_REMOTE_START:-0}"
LTX_SKIP_TUNNEL="${LTX_SKIP_TUNNEL:-0}"
LTX_SKIP_MODEL_CHECK="${LTX_SKIP_MODEL_CHECK:-0}"
LTX_SKIP_NODE_CHECK="${LTX_SKIP_NODE_CHECK:-0}"
LTX_OBJECT_INFO_TIMEOUT_SECS="${LTX_OBJECT_INFO_TIMEOUT_SECS:-90}"
LTX_KEEP_TUNNEL="${LTX_KEEP_TUNNEL:-0}"
LTX_REMOTE_START_TIMEOUT_SECS="${LTX_REMOTE_START_TIMEOUT_SECS:-180}"

SSH_TARGET=""
SSH_CONTROL_SOCKET=""
STARTED_SSH_CONTROL=false
STARTED_TUNNEL=false
BRIDGE_PID=""

usage() {
    print_script_usage "$0" 15
    cat <<EOF

Environment:
  LTX_SSH_HOST                    required unless remote SSH operations are disabled
  LTX_SSH_PORT                    default ${LTX_SSH_PORT}
  LTX_SSH_USER                    default ${LTX_SSH_USER}
  LTX_LOCAL_COMFY_PORT            default ${LTX_LOCAL_COMFY_PORT}
  LTX_REMOTE_COMFY_HOST           default ${LTX_REMOTE_COMFY_HOST}
  LTX_REMOTE_COMFY_PORT           default ${LTX_REMOTE_COMFY_PORT}
  LTX_REMOTE_COMFY_ROOT           default ${LTX_REMOTE_COMFY_ROOT}
  LTX_REMOTE_PYTHON_BIN           default ${LTX_REMOTE_PYTHON_BIN}
  LTX_REMOTE_COMFY_LOG            default ${LTX_REMOTE_COMFY_LOG}
  LTX_REMOTE_HF_ENDPOINT          default ${LTX_REMOTE_HF_ENDPOINT}
  LTX_REMOTE_HF_HOME              default ${LTX_REMOTE_HF_HOME}
  LTX_REMOTE_HF_HUB_CACHE         default ${LTX_REMOTE_HF_HUB_CACHE}
  LTX_REMOTE_HF_XET_CACHE         default ${LTX_REMOTE_HF_XET_CACHE}
  LTX_REMOTE_HF_HUB_DISABLE_XET   default ${LTX_REMOTE_HF_HUB_DISABLE_XET}
  LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT default ${LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT}
  LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS default ${LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS}
  LTX_REMOTE_HTTP_PROXY           optional proxy for remote HF access
  LTX_REMOTE_HTTPS_PROXY          optional proxy for remote HF access
  LTX_REMOTE_ALL_PROXY            optional proxy for remote HF access
  LTX_REMOTE_NO_PROXY             default ${LTX_REMOTE_NO_PROXY}
  LTX_COMFY_BASE_URL              default ${LTX_COMFY_BASE_URL}
  LTX_BRIDGE_HOST                 default ${LTX_BRIDGE_HOST}
  LTX_BRIDGE_PORT                 default ${LTX_BRIDGE_PORT}
  LTX_BRIDGE_API_KEY              default ltx-local
  LTX_BRIDGE_CONFIG               default ${LTX_BRIDGE_CONFIG}
  LTX_SKIP_REMOTE_START=1         skip remote service startup
  LTX_SKIP_TUNNEL=1               skip SSH tunnel creation
  LTX_SKIP_MODEL_CHECK=1          skip remote model file preflight
  LTX_SKIP_NODE_CHECK=1           skip ComfyUI node/object_info preflight
  LTX_OBJECT_INFO_TIMEOUT_SECS    default ${LTX_OBJECT_INFO_TIMEOUT_SECS}
  LTX_KEEP_TUNNEL=1               keep the SSH tunnel after this script exits

Request routing:
  no image fields                                -> H17 text-to-video
  reference_image_urls[0]                      -> H17 single-reference video
  reference_image_urls[0..1]                   -> H21 dual-reference video
  first_frame_image_url                        -> H17 first-frame video
  first_frame_image_url + last_frame_image_url -> H27 first/last-frame video
  first+last + reference_image_urls[0..1]      -> H27 with supplemental reference guides
  quality                                      workflow | draft | standard | high | ultra
  Bridge config controls private LTX inference parameters:
    ${LTX_BRIDGE_CONFIG}

Remote model preflight checks the FP8/FP4 LTX 2.3 setup used by these scripts:
  checkpoints/${LTX_CHECKPOINT_NAME}
  loras/${LTX_LORA_NAME}
  latent_upscale_models/${LTX_UPSCALER_NAME}
  text_encoders/${LTX_TEXT_ENCODER_NAME}
  text_encoders/${ZIMAGE_TEXT_ENCODER_NAME}
  diffusion_models/${ZIMAGE_MODEL_NAME}
  model_patches/${ZIMAGE_CONTROLNET_PATCH_NAME}
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-remote-start)
            LTX_SKIP_REMOTE_START=1
            shift
            ;;
        --no-tunnel)
            LTX_SKIP_TUNNEL=1
            shift
            ;;
        --no-model-check)
            LTX_SKIP_MODEL_CHECK=1
            shift
            ;;
        --no-node-check)
            LTX_SKIP_NODE_CHECK=1
            shift
            ;;
        --keep-tunnel)
            LTX_KEEP_TUNNEL=1
            shift
            ;;
        --panel-url)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                echo -e "${RED}--panel-url requires a URL${NC}"
                exit 1
            fi
            echo -e "${YELLOW}--panel-url is ignored; the bridge now talks directly to ComfyUI.${NC}"
            shift 2
            ;;
        --bridge-port)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                echo -e "${RED}--bridge-port requires a port${NC}"
                exit 1
            fi
            LTX_BRIDGE_PORT="$2"
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown argument: $1${NC}"
            echo "Use --help for usage."
            exit 1
            ;;
    esac
done

if [[ "$LTX_SKIP_REMOTE_START" != "1" || "$LTX_SKIP_TUNNEL" != "1" || "$LTX_SKIP_MODEL_CHECK" != "1" ]]; then
    [[ -n "$LTX_SSH_HOST" ]] || die "LTX_SSH_HOST is required for remote startup, tunnel, or model checks"
fi
if [[ -n "$LTX_SSH_HOST" ]]; then
    SSH_TARGET="${LTX_SSH_USER}@${LTX_SSH_HOST}"
    SSH_CONTROL_SOCKET="$APP_TMP_DIR/ltx-ssh-${LTX_SSH_HOST}-${LTX_SSH_PORT}-$$.sock"
fi

log_step() {
    log_section "$@"
}

fail() {
    die "$@"
}

require_cmd() {
    require_command "$1"
}

http_ok() {
    local url="$1"
    curl -fsS --max-time 3 "$url" >/dev/null 2>&1
}

port_in_use() {
    local port="$1"
    if command -v lsof >/dev/null 2>&1; then
        lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1
    elif command -v nc >/dev/null 2>&1; then
        nc -z 127.0.0.1 "$port" >/dev/null 2>&1
    else
        (echo >"/dev/tcp/127.0.0.1/${port}") >/dev/null 2>&1
    fi
}

bridge_health_url() {
    printf 'http://%s:%s/health\n' "$LTX_BRIDGE_HOST" "$LTX_BRIDGE_PORT"
}

comfy_system_stats_url() {
    printf '%s/system_stats\n' "${LTX_COMFY_BASE_URL%/}"
}

wait_for_comfy() {
    local deadline=$((SECONDS + LTX_REMOTE_START_TIMEOUT_SECS))
    while (( SECONDS < deadline )); do
        if http_ok "$(comfy_system_stats_url)"; then
            return 0
        fi
        sleep 2
    done
    return 1
}

ssh_base_args() {
    printf '%s\n' \
        -p "$LTX_SSH_PORT" \
        -o StrictHostKeyChecking=yes \
        -o ServerAliveInterval=30 \
        -o ServerAliveCountMax=3
}

ssh_with_control_args() {
    ssh_base_args
    printf '%s\n' -S "$SSH_CONTROL_SOCKET"
}

cleanup() {
    local status=$?
    if [[ -n "${BRIDGE_PID:-}" ]] && kill -0 "$BRIDGE_PID" >/dev/null 2>&1; then
        kill "$BRIDGE_PID" >/dev/null 2>&1 || true
        wait "$BRIDGE_PID" >/dev/null 2>&1 || true
    fi
    if [[ "$STARTED_SSH_CONTROL" == "true" && ! ( "$STARTED_TUNNEL" == "true" && "$LTX_KEEP_TUNNEL" == "1" ) ]]; then
        mapfile -t args < <(ssh_with_control_args)
        ssh "${args[@]}" -O exit "$SSH_TARGET" >/dev/null 2>&1 || true
        rm -f "$SSH_CONTROL_SOCKET"
    elif [[ "$STARTED_TUNNEL" == "true" ]]; then
        log_warn "SSH tunnel kept alive by LTX_KEEP_TUNNEL=1"
    fi
    exit "$status"
}
trap cleanup EXIT INT TERM

check_prerequisites() {
    log_step "Checks"
    require_cmd ssh
    require_cmd curl
    require_cmd node

    local node_major
    node_major="$(node -p "Number(process.versions.node.split('.')[0])" 2>/dev/null || echo 0)"
    if [[ "$node_major" -lt 18 ]]; then
        fail "Node.js 18+ is required; current version is $(node --version)"
    fi
    log_ok "ssh/curl/node ready; node $(node --version)"

    if [[ ! -f "$PROJECT_ROOT/scripts/ltx/ltx-comfy-bridge.mjs" ]]; then
        fail "Missing scripts/ltx/ltx-comfy-bridge.mjs"
    fi
}

start_tunnel_if_needed() {
    log_step "SSH tunnel"

    if [[ "$LTX_SKIP_TUNNEL" == "1" ]]; then
        log_warn "Skipped tunnel creation by LTX_SKIP_TUNNEL=1"
        return
    fi

    local comfy_ready=false
    if http_ok "$(comfy_system_stats_url)"; then
        comfy_ready=true
    fi

    if [[ "$comfy_ready" == "true" ]]; then
        log_ok "ComfyUI stats are already reachable at ${LTX_COMFY_BASE_URL%/}"
        return
    fi

    if [[ "$comfy_ready" != "true" ]] && port_in_use "$LTX_LOCAL_COMFY_PORT"; then
        fail "Local port ${LTX_LOCAL_COMFY_PORT} is in use, but ${LTX_COMFY_BASE_URL%/}/system_stats is not reachable"
    fi

    echo -e "  ${CYAN}→${NC} Opening SSH tunnel to ${SSH_TARGET}:${LTX_SSH_PORT}"
    echo -e "  ${YELLOW}SSH will ask for the cloud password if no key is configured.${NC}"

    mapfile -t args < <(ssh_base_args)
    ssh "${args[@]}" \
        -fN -M -S "$SSH_CONTROL_SOCKET" \
        -o ExitOnForwardFailure=yes \
        -L "127.0.0.1:${LTX_LOCAL_COMFY_PORT}:${LTX_REMOTE_COMFY_HOST}:${LTX_REMOTE_COMFY_PORT}" \
        "$SSH_TARGET"

    STARTED_TUNNEL=true
    STARTED_SSH_CONTROL=true
    log_ok "ComfyUI tunnel ready: ${LTX_COMFY_BASE_URL%/} -> ${LTX_REMOTE_COMFY_HOST}:${LTX_REMOTE_COMFY_PORT}"
}

ensure_ssh_control() {
    if [[ "$STARTED_SSH_CONTROL" == "true" ]]; then
        return
    fi

    echo -e "  ${CYAN}→${NC} Opening SSH control session to ${SSH_TARGET}:${LTX_SSH_PORT}"
    echo -e "  ${YELLOW}SSH will ask for the cloud password if no key is configured.${NC}"

    mapfile -t args < <(ssh_base_args)
    ssh "${args[@]}" \
        -fN -M -S "$SSH_CONTROL_SOCKET" \
        "$SSH_TARGET"

    STARTED_SSH_CONTROL=true
}

start_remote_services() {
    log_step "Remote services"

    if [[ "$LTX_SKIP_REMOTE_START" == "1" ]]; then
        log_warn "Skipped remote startup by LTX_SKIP_REMOTE_START=1"
        return
    fi

    ensure_ssh_control
    mapfile -t args < <(ssh_with_control_args)

    echo -e "  ${CYAN}→${NC} Checking ComfyUI on the cloud host"
    ssh "${args[@]}" "$SSH_TARGET" \
        env \
        LTX_REMOTE_COMFY_HOST="$LTX_REMOTE_COMFY_HOST" \
        LTX_REMOTE_COMFY_PORT="$LTX_REMOTE_COMFY_PORT" \
        LTX_REMOTE_COMFY_ROOT="$LTX_REMOTE_COMFY_ROOT" \
        LTX_REMOTE_PYTHON_BIN="$LTX_REMOTE_PYTHON_BIN" \
        LTX_REMOTE_COMFY_LOG="$LTX_REMOTE_COMFY_LOG" \
        LTX_REMOTE_HF_ENDPOINT="$LTX_REMOTE_HF_ENDPOINT" \
        LTX_REMOTE_HF_HOME="$LTX_REMOTE_HF_HOME" \
        LTX_REMOTE_HF_HUB_CACHE="$LTX_REMOTE_HF_HUB_CACHE" \
        LTX_REMOTE_HF_XET_CACHE="$LTX_REMOTE_HF_XET_CACHE" \
        LTX_REMOTE_HF_HUB_DISABLE_XET="$LTX_REMOTE_HF_HUB_DISABLE_XET" \
        LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT="$LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT" \
        LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS="$LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS" \
        LTX_REMOTE_HTTP_PROXY="$LTX_REMOTE_HTTP_PROXY" \
        LTX_REMOTE_HTTPS_PROXY="$LTX_REMOTE_HTTPS_PROXY" \
        LTX_REMOTE_ALL_PROXY="$LTX_REMOTE_ALL_PROXY" \
        LTX_REMOTE_NO_PROXY="$LTX_REMOTE_NO_PROXY" \
        bash -s <<'REMOTE'
set -euo pipefail

comfy_ok=false
comfy_port_busy=false

if curl -fsS --max-time 3 "http://${LTX_REMOTE_COMFY_HOST}:${LTX_REMOTE_COMFY_PORT}/system_stats" >/dev/null 2>&1; then
    comfy_ok=true
fi

if [[ "$comfy_ok" != "true" ]]; then
    if command -v ss >/dev/null 2>&1; then
        if ss -ltn "sport = :${LTX_REMOTE_COMFY_PORT}" | grep -q ":${LTX_REMOTE_COMFY_PORT}"; then
            comfy_port_busy=true
        fi
    elif command -v lsof >/dev/null 2>&1; then
        if lsof -nP -iTCP:"${LTX_REMOTE_COMFY_PORT}" -sTCP:LISTEN >/dev/null 2>&1; then
            comfy_port_busy=true
        fi
    else
        PORT_CHECK_PYTHON="${LTX_REMOTE_PYTHON_BIN:-/root/miniconda3/bin/python}"
        if [[ ! -x "$PORT_CHECK_PYTHON" ]]; then
            PORT_CHECK_PYTHON="$(command -v python || true)"
        fi
        if [[ -n "$PORT_CHECK_PYTHON" ]] && "$PORT_CHECK_PYTHON" - "$LTX_REMOTE_COMFY_HOST" "$LTX_REMOTE_COMFY_PORT" <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
try:
    sock.bind((host, port))
except OSError:
    sys.exit(0)
finally:
    sock.close()
sys.exit(1)
PY
        then
            comfy_port_busy=true
        fi
    fi
fi

cat >/root/start-comfyui-a4n.sh <<'STARTER'
#!/usr/bin/env bash
set -euo pipefail
cd "${LTX_REMOTE_COMFY_ROOT:-/root/ComfyUI}"
PYTHON_BIN="${PYTHON_BIN:-${LTX_REMOTE_PYTHON_BIN:-/root/miniconda3/bin/python}}"
if [[ ! -x "$PYTHON_BIN" ]]; then
  PYTHON_BIN="$(command -v python3 || command -v python)"
fi
export HF_ENDPOINT="${LTX_REMOTE_HF_ENDPOINT:-https://hf-mirror.com}"
export HF_HOME="${LTX_REMOTE_HF_HOME:-/root/autodl-tmp/huggingface}"
export HF_HUB_CACHE="${LTX_REMOTE_HF_HUB_CACHE:-${HF_HOME%/}/hub}"
export HF_XET_CACHE="${LTX_REMOTE_HF_XET_CACHE:-${HF_HOME%/}/xet}"
export HF_HUB_DISABLE_XET="${LTX_REMOTE_HF_HUB_DISABLE_XET:-0}"
export HF_HUB_DOWNLOAD_TIMEOUT="${LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT:-60}"
export HF_XET_NUM_CONCURRENT_RANGE_GETS="${LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS:-8}"
mkdir -p "$HF_HOME" "$HF_HUB_CACHE" "$HF_XET_CACHE"
if [[ -n "${LTX_REMOTE_HTTP_PROXY:-}" ]]; then
  export http_proxy="$LTX_REMOTE_HTTP_PROXY"
  export HTTP_PROXY="$LTX_REMOTE_HTTP_PROXY"
fi
if [[ -n "${LTX_REMOTE_HTTPS_PROXY:-}" ]]; then
  export https_proxy="$LTX_REMOTE_HTTPS_PROXY"
  export HTTPS_PROXY="$LTX_REMOTE_HTTPS_PROXY"
fi
if [[ -n "${LTX_REMOTE_ALL_PROXY:-}" ]]; then
  export all_proxy="$LTX_REMOTE_ALL_PROXY"
  export ALL_PROXY="$LTX_REMOTE_ALL_PROXY"
fi
if [[ -n "${LTX_REMOTE_NO_PROXY:-}" ]]; then
  export no_proxy="$LTX_REMOTE_NO_PROXY"
  export NO_PROXY="$LTX_REMOTE_NO_PROXY"
fi
exec "$PYTHON_BIN" main.py --port "${LTX_REMOTE_COMFY_PORT:-6006}" --listen "${LTX_REMOTE_COMFY_HOST:-127.0.0.1}" --enable-cors-header '*'
STARTER
chmod +x /root/start-comfyui-a4n.sh

if [[ "$comfy_ok" == "true" ]]; then
    echo "comfyui already running"
elif [[ "$comfy_port_busy" == "true" ]]; then
    echo "comfyui port ${LTX_REMOTE_COMFY_PORT} is already occupied; reusing the existing process and waiting for API readiness"
else
    nohup env \
      LTX_REMOTE_COMFY_ROOT="$LTX_REMOTE_COMFY_ROOT" \
      LTX_REMOTE_PYTHON_BIN="$LTX_REMOTE_PYTHON_BIN" \
      LTX_REMOTE_COMFY_HOST="$LTX_REMOTE_COMFY_HOST" \
      LTX_REMOTE_COMFY_PORT="$LTX_REMOTE_COMFY_PORT" \
      LTX_REMOTE_HF_ENDPOINT="$LTX_REMOTE_HF_ENDPOINT" \
      LTX_REMOTE_HF_HOME="$LTX_REMOTE_HF_HOME" \
      LTX_REMOTE_HF_HUB_CACHE="$LTX_REMOTE_HF_HUB_CACHE" \
      LTX_REMOTE_HF_XET_CACHE="$LTX_REMOTE_HF_XET_CACHE" \
      LTX_REMOTE_HF_HUB_DISABLE_XET="$LTX_REMOTE_HF_HUB_DISABLE_XET" \
      LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT="$LTX_REMOTE_HF_HUB_DOWNLOAD_TIMEOUT" \
      LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS="$LTX_REMOTE_HF_XET_NUM_CONCURRENT_RANGE_GETS" \
      LTX_REMOTE_HTTP_PROXY="$LTX_REMOTE_HTTP_PROXY" \
      LTX_REMOTE_HTTPS_PROXY="$LTX_REMOTE_HTTPS_PROXY" \
      LTX_REMOTE_ALL_PROXY="$LTX_REMOTE_ALL_PROXY" \
      LTX_REMOTE_NO_PROXY="$LTX_REMOTE_NO_PROXY" \
      bash /root/start-comfyui-a4n.sh >"$LTX_REMOTE_COMFY_LOG" 2>&1 &
    echo "started comfyui via /root/start-comfyui-a4n.sh"
fi
REMOTE
}

check_remote_models() {
    log_step "Remote model files"

    if [[ "$LTX_SKIP_MODEL_CHECK" == "1" ]]; then
        log_warn "Skipped remote model preflight by LTX_SKIP_MODEL_CHECK=1"
        return
    fi

    ensure_ssh_control
    mapfile -t args < <(ssh_with_control_args)

    if ssh "${args[@]}" "$SSH_TARGET" \
        env \
        LTX_CHECKPOINT_NAME="$LTX_CHECKPOINT_NAME" \
        LTX_LORA_NAME="$LTX_LORA_NAME" \
        LTX_UPSCALER_NAME="$LTX_UPSCALER_NAME" \
        LTX_TEXT_ENCODER_NAME="$LTX_TEXT_ENCODER_NAME" \
        ZIMAGE_MODEL_NAME="$ZIMAGE_MODEL_NAME" \
        ZIMAGE_TEXT_ENCODER_NAME="$ZIMAGE_TEXT_ENCODER_NAME" \
        ZIMAGE_VAE_NAME="$ZIMAGE_VAE_NAME" \
        ZIMAGE_CONTROLNET_PATCH_NAME="$ZIMAGE_CONTROLNET_PATCH_NAME" \
        LTX_REMOTE_COMFY_ROOT="$LTX_REMOTE_COMFY_ROOT" \
        bash -s <<'REMOTE'
set -euo pipefail

required_files=(
  "${LTX_REMOTE_COMFY_ROOT}/models/checkpoints/${LTX_CHECKPOINT_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/loras/${LTX_LORA_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/latent_upscale_models/${LTX_UPSCALER_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/text_encoders/${LTX_TEXT_ENCODER_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/text_encoders/${ZIMAGE_TEXT_ENCODER_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/vae/${ZIMAGE_VAE_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/diffusion_models/${ZIMAGE_MODEL_NAME}"
  "${LTX_REMOTE_COMFY_ROOT}/models/model_patches/${ZIMAGE_CONTROLNET_PATCH_NAME}"
)

missing=0
for file in "${required_files[@]}"; do
  if [[ -f "$file" ]]; then
    size="$(du -hL "$file" 2>/dev/null | awk '{print $1}')"
    echo "OK ${file} ${size}"
  elif [[ -L "$file" ]]; then
    target="$(readlink "$file")"
    echo "BROKEN_LINK ${file} -> ${target}"
    missing=1
  else
    echo "MISSING ${file}"
    missing=1
  fi
done

if [[ -L /.autodl && ! -e /.autodl ]]; then
  echo "WARN_BROKEN_AUTODL_MOUNT /.autodl -> $(readlink /.autodl)"
fi

exit "$missing"
REMOTE
    then
        log_ok "Required LTX model files exist"
        return
    fi

    fail "Remote LTX model files are missing or broken. Restore the cloud model mount/cache first, then rerun this script. The current ComfyUI error is caused by missing model files before GPU inference starts."
}

wait_for_comfy_ready() {
    log_step "ComfyUI readiness"
    if wait_for_comfy; then
        log_ok "ComfyUI API reachable: $(comfy_system_stats_url)"
    else
        fail "ComfyUI did not become reachable within ${LTX_REMOTE_START_TIMEOUT_SECS}s. Check SSH/cloud status and remote log: ${LTX_REMOTE_COMFY_LOG}"
    fi
}

check_comfy_nodes() {
    log_step "ComfyUI nodes"

    if [[ "$LTX_SKIP_NODE_CHECK" == "1" ]]; then
        log_warn "Skipped ComfyUI node preflight by LTX_SKIP_NODE_CHECK=1"
        return
    fi

    local object_info
    local object_info_error
    object_info_error="$(mktemp "$APP_TMP_DIR/object-info-error.XXXXXX")"
    if ! object_info="$(curl -fsS --connect-timeout 5 --max-time "$LTX_OBJECT_INFO_TIMEOUT_SECS" "${LTX_COMFY_BASE_URL%/}/object_info" 2>"$object_info_error")"; then
        local curl_error
        curl_error="$(tr '\n' ' ' <"$object_info_error" | sed 's/[[:space:]]\+/ /g')"
        rm -f "$object_info_error"
        log_warn "Unable to read ComfyUI object_info within ${LTX_OBJECT_INFO_TIMEOUT_SECS}s; skipping node preflight"
        if [[ -n "$curl_error" ]]; then
            log_warn "$curl_error"
        fi
        log_warn "If generation later fails with missing_node_type, rerun after ComfyUI finishes loading or set LTX_OBJECT_INFO_TIMEOUT_SECS=180."
        return
    fi
    rm -f "$object_info_error"

    local result
    if ! result="$(node -e '
const fs = require("node:fs");
const info = JSON.parse(fs.readFileSync(0, "utf8"));
const required = [
  "CheckpointLoaderSimple",
  "CLIPLoader",
  "VAELoader",
  "KSampler",
  "CLIPTextEncode",
  "VAEDecode",
  "SaveImage",
  "LoadImage",
  "Canny",
  "ImageScaleToMaxDimension",
  "ModelPatchLoader",
  "QwenImageDiffsynthControlnet",
  "NunchakuZImageDiTLoader",
  "ModelSamplingAuraFlow",
  "LTXVPreprocess",
  "ManualSigmas",
  "LTXVLatentUpsampler",
  "LTXVAudioVAELoader"
];
const missing = required.filter((name) => !info[name]);
if (missing.length > 0) {
  console.error(`missing ComfyUI nodes: ${missing.join(", ")}`);
  process.exit(1);
}
console.log(`required nodes ready: ${required.length}`);
' <<<"$object_info" 2>&1)"; then
        fail "$result"
    fi

    log_ok "$result"
}

print_a4n_config() {
    cat <<EOF

${BOLD}A4N video API config${NC}
  Provider: OpenAI compatible video
  Base URL: http://${LTX_BRIDGE_HOST}:${LTX_BRIDGE_PORT}/v1
  Model: ltx-2.3
  API Key: ${LTX_BRIDGE_API_KEY}
  Concurrency: 1
  ComfyUI: ${LTX_COMFY_BASE_URL%/}
  GPU stats: ${LTX_COMFY_BASE_URL%/}/system_stats
  Bridge config: ${LTX_BRIDGE_CONFIG}

${BOLD}Supported video request fields${NC}
  reference_image_urls                         one image uses H17; two or more images use H21 first two refs
  first_frame_image_url                        first-frame video
  first_frame_image_url + last_frame_image_url first/last-frame video
  first+last + reference_image_urls            first/last-frame video with up to two supplemental refs
  Private LTX parameters are read from the bridge config, not required in A4N requests.

${BOLD}A4N image API config${NC}
  Provider: OpenAI compatible image
  Base URL: http://${LTX_BRIDGE_HOST}:${LTX_BRIDGE_PORT}/v1
  Model: z-image-turbo
  API Key: ${LTX_BRIDGE_API_KEY}
  Workflow: A4N-ZImage-Turbo-text.json

EOF
}

start_bridge() {
    log_step "Local bridge"

    if http_ok "$(bridge_health_url)"; then
        log_ok "Bridge already reachable: $(bridge_health_url)"
        print_a4n_config
        return 0
    fi

    if port_in_use "$LTX_BRIDGE_PORT"; then
        fail "Bridge port ${LTX_BRIDGE_PORT} is already in use, but $(bridge_health_url) is not healthy"
    fi

    print_a4n_config
    echo -e "${CYAN}━━━ Bridge logs ━━━${NC}"

    LTX_COMFY_BASE_URL="${LTX_COMFY_BASE_URL%/}" \
    LTX_BRIDGE_CONFIG="$LTX_BRIDGE_CONFIG" \
    LTX_BRIDGE_HOST="$LTX_BRIDGE_HOST" \
    LTX_BRIDGE_PORT="$LTX_BRIDGE_PORT" \
    LTX_BRIDGE_API_KEY="$LTX_BRIDGE_API_KEY" \
    LTX_CHECKPOINT_NAME="$LTX_CHECKPOINT_NAME" \
    LTX_LORA_NAME="$LTX_LORA_NAME" \
    LTX_UPSCALER_NAME="$LTX_UPSCALER_NAME" \
    LTX_TEXT_ENCODER_NAME="$LTX_TEXT_ENCODER_NAME" \
    node "$PROJECT_ROOT/scripts/ltx/ltx-comfy-bridge.mjs" &
    BRIDGE_PID=$!
    wait "$BRIDGE_PID"
}

check_prerequisites
start_tunnel_if_needed
start_remote_services
check_remote_models
wait_for_comfy_ready
check_comfy_nodes
start_bridge
