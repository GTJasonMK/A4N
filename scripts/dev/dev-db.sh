#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

COMMAND="${1:-up}"

require_command docker

local_database_config() {
  PROJECT_ROOT="$PROJECT_ROOT" python3 - <<'PY'
import os
import re
from pathlib import Path
from urllib.parse import unquote, urlparse

root = Path(os.environ["PROJECT_ROOT"])
values = {}
env_file = root / ".env"
if env_file.is_file():
    for raw_line in env_file.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key.strip() in {"A4N_DATABASE_URL", "A4N_POSTGRES_PORT"}:
            values[key.strip()] = value.strip().strip('"').strip("'")

for key in ("A4N_DATABASE_URL", "A4N_POSTGRES_PORT"):
    if key in os.environ:
        values[key] = os.environ[key].strip()
database_url = values.get("A4N_DATABASE_URL", "")
postgres_port = values.get("A4N_POSTGRES_PORT", "5432")
if not database_url:
    print("create\ta4n")
    raise SystemExit

parsed = urlparse(database_url)
host = parsed.hostname or ""
port = parsed.port or 5432
database = unquote(parsed.path.lstrip("/"))
if not database or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", database):
    raise SystemExit("A4N_DATABASE_URL 的数据库名必须是简单 PostgreSQL 标识符")
if host in {"localhost", "127.0.0.1", "::1"} and str(port) == postgres_port:
    print(f"create\t{database}")
else:
    print("skip\t远程或非当前端口数据库")
PY
}

wait_for_postgres() {
  for _ in {1..30}; do
    if docker compose exec -T postgres pg_isready -U a4n -d postgres >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  die "PostgreSQL 启动超时"
}

ensure_local_database() {
  local config action database
  config="$(local_database_config)" || die "无法解析本机数据库配置"
  IFS=$'\t' read -r action database <<<"$config"
  if [[ "$action" != "create" ]]; then
    log_info "跳过本机数据库创建：$database"
    return 0
  fi

  wait_for_postgres
  if docker compose exec -T postgres psql -U a4n -d postgres -Atqc \
    "SELECT 1 FROM pg_database WHERE datname = '$database'" | grep -qx 1; then
    log_info "数据库已存在: $database"
    return 0
  fi
  docker compose exec -T postgres createdb -U a4n "$database"
  log_ok "数据库已创建: $database"
}

case "$COMMAND" in
  up)
    docker compose up -d postgres
    ensure_local_database
    ;;
  down)
    docker compose down
    ;;
  status)
    docker compose ps postgres
    ;;
  *)
    die "用法: $0 {up|down|status}"
    ;;
esac
