replace_remote_delta() {
    log_section "远端增量替换（不停止 Host）"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" "$REMOTE_PACKAGE" "$REMOTE_SERVICE" <<'REMOTE' || return $?
set -euo pipefail
app="$1"
package_path="$2"
service="$3"
stage="$app/.deploy-delta"
state="$app/.deploy-state/current.json"
rm -rf "$stage"
mkdir -p "$stage"
cleanup_delta_stage() {
    rm -rf "$stage"
}
trap cleanup_delta_stage EXIT
tar -xzf "$package_path" -C "$stage"
test -f "$stage/delta-manifest.json"
test -f "$stage/release-manifest.json"
test -f "$stage/release-manifest.sha256"

changed_files="$stage/.changed-files"
python3 - "$stage/delta-manifest.json" "$stage" "$state" > "$changed_files" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1]).resolve()
root = manifest_path.parent
delta = json.loads(manifest_path.read_text())
state_path = pathlib.Path(sys.argv[3])
state = json.loads(state_path.read_text())
if delta.get("schema_version") != 1 or delta.get("mode") != "delta":
    raise SystemExit("invalid delta manifest")
if state.get("release_id") != delta.get("base_release_id"):
    raise SystemExit(42)
release_manifest = root / "release-manifest.json"
release_id = hashlib.sha256(release_manifest.read_bytes()).hexdigest()
if release_id != delta.get("target_release_id"):
    raise SystemExit("delta target release does not match release manifest")
seen = set()
for entry in delta.get("files", []):
    relative = entry.get("path")
    kind = entry.get("kind")
    if not isinstance(relative, str) or relative in seen:
        raise SystemExit("delta file list is invalid")
    seen.add(relative)
    candidate = pathlib.PurePosixPath(relative)
    if candidate.is_absolute() or any(part in {"", ".", ".."} for part in candidate.parts):
        raise SystemExit(f"unsafe delta path: {relative}")
    if kind == "directory":
        if relative not in {"static", "prompts"}:
            raise SystemExit(f"unsupported delta directory: {relative}")
        if not (root / relative).is_dir():
            raise SystemExit(f"missing delta directory: {relative}")
    elif kind == "file":
        if not relative.startswith("bin/") or not (root / pathlib.Path(*candidate.parts)).is_file():
            raise SystemExit(f"missing or unsafe delta file: {relative}")
    else:
        raise SystemExit(f"invalid delta file kind: {relative}")
    print(f"{kind}\t{relative}")
PY

current_manifest_id="$(sha256sum "$app/release-manifest.json" | cut -d' ' -f1)"
base_release_id="$(python3 - "$state" <<'PY'
import json
import sys
print(json.loads(open(sys.argv[1]).read()).get("release_id", ""))
PY
)"
expected_base_id="$(python3 - "$stage/delta-manifest.json" <<'PY'
import json
import sys
print(json.loads(open(sys.argv[1]).read()).get("base_release_id", ""))
PY
)"
if [[ -z "$base_release_id" || "$current_manifest_id" != "$base_release_id" || "$base_release_id" != "$expected_base_id" ]]; then
    echo "远端 release 基线不一致，需要全量部署" >&2
    rm -rf "$stage"
    exit 42
fi

ts="$(date +%Y%m%d%H%M%S)"
backup="$app/backups/deploy-$ts"
mkdir -p "$backup"
applied=false

restore_delta() {
    local status=$?
    if [[ "$applied" == true && "$status" -ne 0 ]]; then
        while IFS=$'\t' read -r kind relative; do
            [[ -n "$relative" ]] || continue
            if [[ "$kind" == directory ]]; then
                rm -rf "$app/$relative"
                [[ -d "$backup/$relative" ]] && cp -a "$backup/$relative" "$app/$relative"
            elif [[ -f "$backup/$relative" ]]; then
                install -m 755 "$backup/$relative" "$app/$relative"
            else
                rm -f "$app/$relative"
            fi
        done < "$changed_files"
        [[ -f "$backup/release-manifest.json" ]] && install -m 644 "$backup/release-manifest.json" "$app/release-manifest.json"
        [[ -f "$backup/release-manifest.sha256" ]] && install -m 644 "$backup/release-manifest.sha256" "$app/release-manifest.sha256"
        sudo systemctl restart "$service" >/dev/null 2>&1 || true
    fi
    rm -rf "$stage"
    exit "$status"
}
trap restore_delta EXIT

cp -a "$app/release-manifest.json" "$backup/release-manifest.json"
cp -a "$app/release-manifest.sha256" "$backup/release-manifest.sha256"
while IFS=$'\t' read -r kind relative; do
    [[ -n "$relative" ]] || continue
    if [[ "$kind" == directory ]]; then
        [[ -d "$app/$relative" ]] && cp -a "$app/$relative" "$backup/$relative"
    elif [[ -f "$app/$relative" ]]; then
        mkdir -p "$backup/$(dirname "$relative")"
        cp -a "$app/$relative" "$backup/$relative"
    fi
done < "$changed_files"

applied=true
while IFS=$'\t' read -r kind relative; do
    [[ -n "$relative" ]] || continue
    if [[ "$kind" == directory ]]; then
        rm -rf "$app/$relative.new"
        cp -a "$stage/$relative" "$app/$relative.new"
        rm -rf "$app/$relative"
        mv "$app/$relative.new" "$app/$relative"
    else
        destination="$app/$relative"
        mkdir -p "$(dirname "$destination")"
        install -m 755 "$stage/$relative" "$destination.new"
        mv "$destination.new" "$destination"
    fi
done < "$changed_files"
install -m 644 "$stage/release-manifest.json" "$app/release-manifest.json.new"
mv "$app/release-manifest.json.new" "$app/release-manifest.json"
install -m 644 "$stage/release-manifest.sha256" "$app/release-manifest.sha256.new"
mv "$app/release-manifest.sha256.new" "$app/release-manifest.sha256"

python3 - "$app" "$app/release-manifest.json" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
manifest = json.loads(pathlib.Path(sys.argv[2]).read_text())
for entry in manifest["binaries"] + manifest["tools"] + manifest["runtime_files"]:
    relative = entry["path"]
    candidate = pathlib.PurePosixPath(relative)
    if candidate.is_absolute() or not relative.startswith("bin/") or any(part in {"", ".", ".."} for part in candidate.parts):
        raise SystemExit(f"unsafe release path: {relative}")
    path = (root / pathlib.Path(*candidate.parts)).resolve()
    if root not in path.parents or not path.is_file():
        raise SystemExit(f"missing release path: {relative}")
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual.lower() != entry["sha256"].lower():
        raise SystemExit(f"release checksum mismatch: {relative}")
PY
(cd "$app" && sha256sum -c release-manifest.sha256 >/dev/null)
systemctl is-active --quiet "$service"
health="$(curl -fsS --max-time 10 http://127.0.0.1:8080/api/health)"
[[ "$health" == "ok" ]] || { echo "增量替换后 Host health 异常: $health" >&2; exit 1; }
echo "backup=$backup"
echo "release_id=$(sha256sum "$app/release-manifest.json" | cut -d' ' -f1)"
echo "health=$health"
trap - EXIT
rm -rf "$stage"
REMOTE
    log_ok "远端增量替换与 Host health 校验完成"
}
prepare_active_state() {
    ruby "$DEPLOY_PLAN_TOOL" record \
        --plan "$DEPLOY_PLAN_PATH" \
        --manifest "$PACKAGE_DIR/release-manifest.json" \
        --output "$DEPLOY_STATE_JSON_PATH" \
        --status active \
        --deployment-id "$DEPLOY_ID" \
        --started-at "$DEPLOY_STARTED_AT" \
        --remote "$REMOTE" \
        --remote-dir "$REMOTE_DIR" \
        --service "$REMOTE_SERVICE" \
        --public-app-url "$PUBLIC_APP_URL" \
        --public-base-path "$PUBLIC_BASE_PATH" \
        --api-base-url "$API_BASE_URL"
}

sync_remote_state() {
    log_section "更新远端部署状态"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" <<'REMOTE'
set -euo pipefail
app="$1"
mkdir -p "$app/.deploy-state"
REMOTE
    scp "${SSH_OPTS[@]}" "$DEPLOY_STATE_JSON_PATH" "$REMOTE:$REMOTE_STATE_PATH.tmp"
    ssh "${SSH_OPTS[@]}" "$REMOTE" 'bash -s' -- "$REMOTE_DIR" <<'REMOTE'
set -euo pipefail
app="$1"
mv "$app/.deploy-state/current.json.tmp" "$app/.deploy-state/current.json"
REMOTE
    log_ok "远端部署状态已原子更新"
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

rm -rf "$app/.deploy-new" "$app/.deploy-delta"
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
health=""
for _ in {1..60}; do
    if health="$(curl -fsS --connect-timeout 1 --max-time 1 http://127.0.0.1:8080/api/health 2>/dev/null)" \
        && [[ "$health" == "ok" ]]; then
        break
    fi
    health=""
    sleep 1
done
if [[ "$health" != "ok" ]]; then
    echo "内部 health 未在 60 秒内就绪: ${health:-连接失败}" >&2
    systemctl status "$service" --no-pager --lines=80 >&2 || true
    journalctl -u "$service" --since "2 minutes ago" --no-pager | tail -n 120 >&2 || true
    exit 1
fi
echo "service=$service"
echo "sidecars=$#"
echo "health=$health"
REMOTE
    log_ok "${#SERVICE_BINARIES[@]} 个 sidecar 均在运行，内部 health=ok"
}
