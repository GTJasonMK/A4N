#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

entry_binary="${A4N_DEV_ENTRY_BINARY:-a4n-tauri}"
build_mode="${A4N_DEV_BUILD_MODE:-remote}"
full_host="${A4N_DEV_BUILD_FULL_HOST:-0}"
managed_sidecars="${A4N_DEV_BUILD_MANAGED_SIDECARS:-0}"
include_cli="${A4N_DEV_BUILD_INCLUDE_CLI:-0}"

case "$entry_binary" in
  a4n-tauri)
    entry_package=a4n-tauri
    ;;
  a4n-server)
    entry_package=a4n-server
    ;;
  a4n)
    entry_package=a4n-cli
    ;;
  a4n-migration)
    entry_package=a4n-migration
    ;;
  *) die "不支持的本地运行入口: $entry_binary" ;;
esac

runner_args=("$@")
if [[ "${runner_args[0]:-}" == run ]]; then
  runner_args=("${runner_args[@]:1}")
fi

qualify_features() {
  local feature result=""
  local -a features=()
  IFS=',' read -r -a features <<<"$1"
  for feature in "${features[@]}"; do
    [[ -n "$feature" ]] || continue
    if [[ "$feature" != */* && "$feature" != dep:* ]]; then
      feature="$entry_package/$feature"
    fi
    result="${result:+$result,}$feature"
  done
  printf '%s' "$result"
}

cargo_args=()
app_args=()
target_dir="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
no_default_features=false
index=0
while ((index < ${#runner_args[@]})); do
  arg="${runner_args[$index]}"
  case "$arg" in
    --)
      app_args=("${runner_args[@]:$((index + 1))}")
      break
      ;;
    --target-dir)
      [[ $((index + 1)) -lt ${#runner_args[@]} ]] || die "--target-dir 缺少目录"
      target_dir="${runner_args[$((index + 1))]}"
      index=$((index + 2))
      continue
      ;;
    --target-dir=*)
      target_dir="${arg#*=}"
      index=$((index + 1))
      continue
      ;;
    --features|-F)
      [[ $((index + 1)) -lt ${#runner_args[@]} ]] || die "$arg 缺少 feature"
      cargo_args+=(--features "$(qualify_features "${runner_args[$((index + 1))]}")")
      index=$((index + 2))
      continue
      ;;
    --features=*)
      cargo_args+=(--features "$(qualify_features "${arg#*=}")")
      index=$((index + 1))
      continue
      ;;
    --no-default-features) no_default_features=true ;;
  esac
  cargo_args+=("$arg")
  index=$((index + 1))
done

packages=()
add_package() {
  local candidate existing
  candidate="$1"
  for existing in "${packages[@]}"; do
    [[ "$existing" == "$candidate" ]] && return 0
  done
  packages+=("$candidate")
}

add_package "$entry_package"
if [[ "$entry_binary" == a4n-server || "$entry_binary" == a4n || "$entry_binary" == a4n-migration ]]; then
  build_mode=local-server
fi
case "$build_mode" in
  local-server)
    for package in a4n-migration a4n-control a4n-edge a4n-server; do
      add_package "$package"
    done
    worker_package_lines="$(stateful_worker_packages)" \
      || die "读取 stateful worker catalog 失败"
    mapfile -t worker_packages <<<"$worker_package_lines"
    ((${#worker_packages[@]} > 0)) || die "stateful worker catalog 为空"
    for package in "${worker_packages[@]}"; do
      add_package "$package"
    done
    if [[ "$managed_sidecars" == 1 && "$full_host" != 1 ]]; then
      sidecar_package_lines="$(sidecar_catalog_binaries)" \
        || die "读取 sidecar catalog 失败"
      mapfile -t sidecar_packages <<<"$sidecar_package_lines"
      for package in "${sidecar_packages[@]}"; do
        [[ -n "$package" ]] && add_package "$package"
      done
    fi
    ;;
  native)
    worker_package_lines="$(stateful_worker_packages)" \
      || die "读取 stateful worker catalog 失败"
    mapfile -t worker_packages <<<"$worker_package_lines"
    for package in "${worker_packages[@]}"; do
      add_package "$package"
    done
    sidecar_package_lines="$(sidecar_catalog_binaries)" \
      || die "读取 sidecar catalog 失败"
    mapfile -t sidecar_packages <<<"$sidecar_package_lines"
    for package in "${sidecar_packages[@]}"; do
      [[ -n "$package" ]] && add_package "$package"
    done
    ;;
  remote) ;;
  *) die "无效的本地构建模式: $build_mode" ;;
esac
[[ "$include_cli" == 1 ]] && add_package a4n-cli
mapfile -t packages < <(printf '%s\n' "${packages[@]}" | LC_ALL=C sort)

# Build the host and runtime binaries in one feature graph; alternating Cargo package selections
# rebuilds shared dependency fingerprints (including thiserror's proc-macro dependency graph).
build_command=("$PROJECT_ROOT/scripts/cargo.sh" build --manifest-path "$PROJECT_ROOT/Cargo.toml")
build_command+=("${cargo_args[@]}")
for package in "${packages[@]}"; do
  build_command+=(-p "$package")
done
if $no_default_features; then
  default_feature_package_lines="$(
    cargo metadata --locked --no-deps --format-version 1 --manifest-path "$PROJECT_ROOT/Cargo.toml" |
      node -e '
        const selected = new Set(process.argv.slice(1));
        const metadata = JSON.parse(require("fs").readFileSync(0, "utf8"));
        for (const pkg of metadata.packages) {
          if (selected.has(pkg.name) && Object.prototype.hasOwnProperty.call(pkg.features, "default")) {
            console.log(pkg.name);
          }
        }
      ' "${packages[@]}"
  )" || die "读取 Cargo 默认 feature 失败"
  mapfile -t default_feature_packages <<<"$default_feature_package_lines"
  for package in "${default_feature_packages[@]}"; do
    [[ "$package" == "$entry_package" ]] || build_command+=(--features "$package/default")
  done
fi
if [[ "$build_mode" == local-server && "$full_host" == 1 ]]; then
  build_command+=(--features a4n-server/embedded-capabilities)
fi

[[ "$target_dir" == /* ]] || target_dir="$PWD/$target_dir"
export CARGO_TARGET_DIR="$target_dir"
export CARGO_ARTIFACTS_DISABLED=1
"${build_command[@]}"
[[ "${A4N_DEV_BUILD_ONLY:-0}" == 1 ]] && exit 0

profile=debug
target_triple="${CARGO_BUILD_TARGET:-}"
for ((index = 0; index < ${#cargo_args[@]}; index++)); do
  case "${cargo_args[$index]}" in
    --release) profile=release ;;
    --profile)
      [[ $((index + 1)) -lt ${#cargo_args[@]} ]] || die "--profile 缺少名称"
      profile="${cargo_args[$((index + 1))]}"
      index=$((index + 1))
      ;;
    --profile=*) profile="${cargo_args[$index]#*=}" ;;
    --target)
      [[ $((index + 1)) -lt ${#cargo_args[@]} ]] || die "--target 缺少目标"
      target_triple="${cargo_args[$((index + 1))]}"
      index=$((index + 1))
      ;;
    --target=*) target_triple="${cargo_args[$index]#*=}" ;;
  esac
done
[[ "$profile" == dev ]] && profile=debug
target_profile_dir="$target_dir"
[[ -n "$target_triple" ]] && target_profile_dir+="/$target_triple"
target_profile_dir+="/$profile"

binary="$target_profile_dir/$entry_binary"
[[ -x "$binary" ]] || die "Cargo 未生成可执行文件: $binary"

case "${OSTYPE:-}" in
  linux*) loader_var=LD_LIBRARY_PATH ;;
  darwin*) loader_var=DYLD_LIBRARY_PATH ;;
  *) loader_var="" ;;
esac
if [[ -n "$loader_var" ]]; then
  library_dirs=("$target_profile_dir/deps")
  for directory in "$target_profile_dir"/build/*/out; do
    [[ -d "$directory" ]] && library_dirs+=("$directory")
  done
  [[ -n "${!loader_var:-}" ]] && library_dirs+=("${!loader_var}")
  library_path="$(IFS=:; printf '%s' "${library_dirs[*]}")"
  export "$loader_var=$library_path"
fi

unset A4N_DEV_BUILD_MODE A4N_DEV_BUILD_FULL_HOST A4N_DEV_BUILD_MANAGED_SIDECARS
unset A4N_DEV_BUILD_INCLUDE_CLI A4N_DEV_ENTRY_BINARY A4N_DEV_BUILD_ONLY
exec "$binary" "${app_args[@]}"
