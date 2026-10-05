#!/usr/bin/env python3
"""Check cloud deployment error propagation and container build boundaries locally."""

from __future__ import annotations

import fcntl
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CLOUD = ROOT / "scripts/deploy/deploy-cloud.sh"
REMOTE = CLOUD.parent / "deploy-cloud/remote.sh"


class DeployCloudTest(unittest.TestCase):
    def test_frontend_build_uses_an_isolated_package_directory(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-cloud-frontend-") as directory:
            temp = Path(directory)
            frontend = temp / "frontend"
            shared_dist = frontend / "dist"
            shared_dist.mkdir(parents=True)
            (shared_dist / "index.html").write_text("shared output\n", encoding="utf-8")
            cache = temp / "target/deploy-cloud"
            result = subprocess.run(
                ["bash", "-c", r'''
set -euo pipefail
source "$1"
FRONTEND_DIR="$2"
CACHE_ROOT="$3"
PUBLIC_BASE_PATH=/a4n
API_BASE_URL=/a4n/api
ensure_frontend_dependencies() { :; }
log_section() { :; }
log_ok() { :; }
die() { printf '%s\\n' "$*" >&2; exit 1; }
npm() {
    [[ "$*" == "run build -- --outDir $CACHE_ROOT/frontend-dist" ]]
    [[ "$VITE_A4N_BASE_PATH" == /a4n && "$VITE_A4N_API_BASE_URL" == /a4n/api ]]
    mkdir -p "$CACHE_ROOT/frontend-dist"
    printf '<script src="/a4n/assets/app.js"></script>\\n' > "$CACHE_ROOT/frontend-dist/index.html"
}
build_frontend
[[ "$FRONTEND_PACKAGE_DIST" == "$CACHE_ROOT/frontend-dist" ]]
grep -Fq '/a4n/assets/' "$FRONTEND_PACKAGE_DIST/index.html"
[[ "$(cat "$FRONTEND_DIR/dist/index.html")" == "shared output" ]]
''', "test", str(CLOUD.parent / "deploy-cloud/build.sh"), str(frontend), str(cache)],
                cwd=ROOT, text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_smoke_checks_served_assets_without_local_frontend_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-cloud-smoke-") as directory:
            requests = Path(directory) / "requests"
            result = subprocess.run(
                ["bash", "-c", r'''
set -euo pipefail
source "$1"
REQUEST_LOG="$2"
RUN_SMOKE=true
PUBLIC_ORIGIN=https://fixture.invalid
PUBLIC_BASE_PATH=/a4n
PUBLIC_APP_URL="$PUBLIC_ORIGIN$PUBLIC_BASE_PATH"
ROOT_EXPECTED_STATUS=any
FRONTEND_DIST=/does-not-exist
log_section() { :; }
log_ok() { :; }
die() { printf '%s\n' "$*" >&2; exit 1; }
curl() {
    local url="${!#}"
    printf '%s\n' "$url" >> "$REQUEST_LOG"
    if [[ "$*" == *'%{http_code}'* ]]; then
        case "$url" in
            */temp-media/*) printf 404 ;;
            */a4n) printf 308 ;;
            *) printf 200 ;;
        esac
    elif [[ "$url" == */api/health ]]; then
        printf ok
    else
        printf '<script src="/a4n/assets/served.js"></script><script src="/a4n/assets/second.js"></script>'
    fi
}
smoke_test
''', "test", str(CLOUD.parent / "deploy-cloud/finalize.sh"), str(requests)],
                cwd=ROOT, text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            urls = requests.read_text().splitlines()
            self.assertIn("https://fixture.invalid/a4n/assets/served.js", urls)
            self.assertNotIn("https://fixture.invalid/a4n/assets/second.js", urls)

    def test_smoke_rejects_served_asset_with_wrong_base_path(self) -> None:
        result = subprocess.run(
            ["bash", "-c", r'''
set -euo pipefail
source "$1"
RUN_SMOKE=true
PUBLIC_ORIGIN=https://fixture.invalid
PUBLIC_BASE_PATH=/a4n
PUBLIC_APP_URL="$PUBLIC_ORIGIN$PUBLIC_BASE_PATH"
ROOT_EXPECTED_STATUS=any
log_section() { :; }
log_ok() { :; }
die() { printf '%s\n' "$*" >&2; exit 1; }
curl() {
    if [[ "$*" == *'%{http_code}'* ]]; then
        if [[ "$*" == *temp-media* ]]; then
            printf 404
        elif [[ "$*" == *fixture.invalid/a4n ]]; then
            printf 308
        else
            printf 200
        fi
    elif [[ "$*" == */api/health* ]]; then
        printf ok
    else
        printf '<script type="module" src="/assets/served.js"></script>'
    fi
}
if (smoke_test); then
    exit 1
else
    status=$?
fi
[[ "$status" == 1 ]]
''', "test", str(CLOUD.parent / "deploy-cloud/finalize.sh")],
            cwd=ROOT, text=True, capture_output=True, timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("JS asset 路径不匹配 /a4n/assets", result.stderr)

    def test_help_and_lock_conflicts_leave_another_build_untouched(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-deploy-lock-") as directory:
            temp = Path(directory)
            target = temp / "target"
            cache = target / "deploy-cloud"
            cache.mkdir(parents=True)
            cidfile = cache / "build-container.cid"
            cidfile.write_text("a" * 64, encoding="utf-8")
            fake_bin = temp / "bin"
            fake_bin.mkdir()
            docker = fake_bin / "docker"
            docker.write_text(
                '#!/usr/bin/env python3\nimport os, pathlib\npathlib.Path(os.environ["A4N_TEST_DOCKER_LOG"]).write_text("called")\nprint("builder-fixture")\n',
                encoding="utf-8",
            )
            docker.chmod(0o755)
            environment = {key: value for key, value in os.environ.items() if not key.startswith("A4N_")}
            environment.update({
                "PATH": f"{fake_bin}:{environment['PATH']}",
                "CARGO_TARGET_DIR": str(target),
                "A4N_DEPLOY_ENV_FILE": "/dev/null",
                "A4N_DEPLOY_HOST": "fixture.invalid",
                "A4N_DEPLOY_SSH_KEY": str(temp / "unused-key"),
                "A4N_DEPLOY_PUBLIC_ORIGIN": "https://fixture.invalid",
                "A4N_DEPLOY_BUILDER_IMAGE": "builder-fixture",
                "A4N_TEST_DOCKER_LOG": str(temp / "docker.log"),
            })
            with (target / "deploy-cloud.lock").open("w") as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                for args, status in ((["--help"], 0), ([], 1)):
                    result = subprocess.run([str(CLOUD), *args], cwd=ROOT, env=environment,
                                            text=True, capture_output=True, timeout=10)
                    self.assertEqual(result.returncode, status, result.stdout + result.stderr)
                    self.assertTrue(cidfile.exists())
                    self.assertFalse((temp / "docker.log").exists())

    def test_builder_image_does_not_upload_the_build_cache_as_context(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-builder-context-") as directory:
            temp = Path(directory)
            result = subprocess.run(
                ["bash", "-c", r'''
set -euo pipefail
source "$1"
CACHE_ROOT="$2"
RUST_BASE_IMAGE=rust-fixture
UBUNTU_BASE_IMAGE=ubuntu-fixture
BUILDER_IMAGE=builder-fixture
REBUILD_BUILDER=true
log_section() { :; }
log_info() { :; }
docker() {
    if [[ "$1" == build ]]; then
        printf '%s\n' "$@" > "$CACHE_ROOT/args"
        cat > "$CACHE_ROOT/context"
    fi
}
ensure_builder_image
''', "test", str(CLOUD.parent / "deploy-cloud/build.sh"), str(temp)],
                cwd=ROOT, text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual((temp / "args").read_text().splitlines(), ["build", "-t", "builder-fixture", "-"])
            self.assertIn("FROM rust-fixture", (temp / "context").read_text())

    def test_incremental_ssh_status_survives_a_conditional_caller(self) -> None:
        for status in (0, 42, 255):
            with self.subTest(status=status):
                result = subprocess.run(
                    ["bash", "-c", r'''
set -euo pipefail
source "$1"
SIMULATED_STATUS="$2"
SSH_OPTS=()
REMOTE=fixture.invalid
REMOTE_DIR=/unused/app
REMOTE_PACKAGE=/unused/package.tar.gz
REMOTE_SERVICE=fixture.service
log_section() { :; }
log_ok() { printf 'success-log\n'; }
ssh() { return "$SIMULATED_STATUS"; }
if replace_remote_delta; then
    result=0
else
    result=$?
fi
exit "$result"
''', "test", str(REMOTE), str(status)],
                    cwd=ROOT, text=True, capture_output=True, timeout=10,
                )
                self.assertEqual(result.returncode, status, result.stdout + result.stderr)
                self.assertEqual("success-log" in result.stdout, status == 0)

    def test_container_build_uses_an_isolated_target_and_callers_identity(self) -> None:
        for offline in ("true", "false"):
            with self.subTest(offline=offline), tempfile.TemporaryDirectory(prefix="a4n-cloud-build-") as directory:
                temp = Path(directory)
                fake_bin = temp / "bin"
                fake_bin.mkdir()
                for name in ("registry", "git"):
                    (temp / "host-cargo" / name).mkdir(parents=True)
                docker_log = temp / "docker.json"
                docker = fake_bin / "docker"
                docker.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
pathlib.Path(os.environ["A4N_TEST_DOCKER_LOG"]).write_text(json.dumps(args))
for index, value in enumerate(args):
    if value == "-v" and args[index + 1].endswith(":/cargo-target"):
        target = pathlib.Path(args[index + 1].removesuffix(":/cargo-target")) / "release"
        target.mkdir(parents=True, exist_ok=True)
        for name in ("a4n-server", "a4n-component-host"):
            binary = target / name
            binary.write_text("fixture")
            binary.chmod(0o755)
''', encoding="utf-8")
                docker.chmod(0o755)
                environment = {key: value for key, value in os.environ.items() if not key.startswith("A4N_")}
                environment.update({
                    "PATH": f"{fake_bin}:{environment['PATH']}",
                    "CARGO_TARGET_DIR": str(temp / "native-target"),
                    "CARGO_HOME": str(temp / "host-cargo"),
                    "A4N_DEPLOY_ENV_FILE": "/dev/null",
                    "A4N_DEPLOY_HOST": "fixture.invalid",
                    "A4N_DEPLOY_SSH_KEY": str(temp / "unused-key"),
                    "A4N_DEPLOY_PUBLIC_ORIGIN": "https://fixture.invalid",
                    "A4N_DEPLOY_RECORD_DIR": str(temp / "records"),
                    "A4N_ONNXRUNTIME_ROOT": str(temp / "onnx"),
                    "A4N_DEPLOY_CARGO_OFFLINE": offline,
                    "A4N_TEST_DOCKER_LOG": str(docker_log),
                })
                result = subprocess.run(
                    ["bash", "-c", r'''
set -euo pipefail
entry="$1"
shift
source "$entry"
log_section() { :; }
log_info() { :; }
log_ok() { :; }
ensure_builder_image() { :; }
BUILD_PACKAGES=(a4n-server a4n-component-host)
TOOL_SPECS=(a4n-component-host:a4n-component-host)
build_backend
[[ "$CARGO_TARGET_DIR" == "$BUILD_TARGET_DIR" ]]
''', "test", str(CLOUD)],
                    cwd=ROOT, env=environment, text=True, capture_output=True, timeout=10,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                args = json.loads(docker_log.read_text(encoding="utf-8"))
                self.assertEqual(args[args.index("--user") + 1], f"{os.getuid()}:{os.getgid()}")
                mounts = [args[index + 1] for index, value in enumerate(args) if value == "-v"]
                cache = temp / "native-target/deploy-cloud"
                self.assertIn(f"{temp / 'native-target'}:/cargo-target", mounts)
                self.assertIn(f"{cache / 'cargo-home'}:/cargo-home", mounts)
                self.assertIn(f"{ROOT}:/work:ro", mounts)
                self.assertIn("CARGO_HOME=/cargo-home", args)
                self.assertTrue((temp / "native-target/release/a4n-server").is_file())
                self.assertTrue((temp / "native-target/release/a4n-component-host").is_file())
                self.assertEqual("--offline" in args[-1], offline == "true")
                if offline == "true":
                    self.assertIn(f"{temp / 'host-cargo/registry'}:/cargo-home/registry:ro", mounts)
                    self.assertIn(f"{temp / 'host-cargo/git'}:/cargo-home/git:ro", mounts)

    def test_upload_uses_rsync_progress_and_atomic_replace(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-cloud-upload-") as directory:
            temp = Path(directory)
            package = temp / "package.tar.gz"
            package.write_bytes(b"deployment-package")
            package_sha256 = subprocess.check_output(["sha256sum", str(package)], text=True).split()[0]
            package.with_name(f"{package.name}.sha256").write_text(
                f"{package_sha256}  {package.name}\n", encoding="utf-8"
            )
            remote_dir = temp / "remote"
            remote_dir.mkdir()
            previous_package = remote_dir / "a4n-deploy.tar.gz"
            previous_package.write_bytes(b"previous-deployment-package")
            args_file = temp / "rsync.args"
            basis_file = temp / "rsync.basis"
            result = subprocess.run(
                ["bash", "-c", r'''
set -euo pipefail
source "$1"
PACKAGE_PATH="$2"
REMOTE=fixture.invalid
REMOTE_PACKAGE="$3/a4n-deploy.tar.gz"
PACKAGE_SIZE=18K
SSH_OPTS=()
RSYNC_ARGS_FILE="$4"
BASIS_FILE="$5"
log_section() { :; }
log_info() { printf '%s\n' "$*"; }
log_ok() { :; }
die() { printf '%s\n' "$*" >&2; exit 1; }
ssh() {
    local command="${!#}"
    case "$command" in
        "command -v rsync"*) return 0 ;;
        *) bash -c "$command" ;;
    esac
}
rsync() {
    printf '%s\n' "$@" > "$RSYNC_ARGS_FILE"
    local args=("$@")
    local source="${args[${#args[@]}-2]}"
    local destination="${args[${#args[@]}-1]}"
    destination="${destination#fixture.invalid:}"
    destination="${destination#\'}"
    destination="${destination%\'}"
    cat "$destination" > "$BASIS_FILE"
    mkdir -p "$(dirname "$destination")"
    cp "$source" "$destination"
}
upload_package
test -f "$3/a4n-deploy.tar.gz"
test ! -e "$3/a4n-deploy.tar.gz.part"
grep -Fx -- '--info=progress2' "$4"
grep -Fx -- '--partial' "$4"
''', "test", str(CLOUD.parent / "deploy-cloud/build.sh"), str(package), str(remote_dir), str(args_file), str(basis_file)],
                cwd=ROOT, text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("rsync", result.stdout)
            self.assertEqual((temp / "rsync.basis").read_bytes(), b"previous-deployment-package")

    def test_upload_does_not_stream_after_ssh_probe_failure(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-cloud-upload-ssh-failure-") as directory:
            temp = Path(directory)
            package = temp / "package.tar.gz"
            package.write_bytes(b"deployment-package")
            package.with_name(f"{package.name}.sha256").write_text(
                f"{subprocess.check_output(['sha256sum', str(package)], text=True).split()[0]}  {package.name}\n",
                encoding="utf-8",
            )
            stream_marker = temp / "stream-started"
            result = subprocess.run(
                ["bash", "-c", r'''
set -euo pipefail
source "$1"
PACKAGE_PATH="$2"
REMOTE=fixture.invalid
REMOTE_PACKAGE=/unused/a4n-deploy.tar.gz
SSH_OPTS=()
log_section() { :; }
log_info() { :; }
log_warn() { :; }
die() { printf '%s\\n' "$*" >&2; exit 1; }
ssh() { return 255; }
sleep() { :; }
dd() { : > "$STREAM_MARKER"; }
STREAM_MARKER="$3"
upload_package
''', "test", str(CLOUD.parent / "deploy-cloud/build.sh"), str(package), str(stream_marker)],
                cwd=ROOT, text=True, capture_output=True, timeout=10,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("SSH 检查远端 rsync 失败", result.stderr)
            self.assertFalse(stream_marker.exists())

    def test_upload_falls_back_to_dd_with_atomic_replace(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-cloud-upload-dd-") as directory:
            temp = Path(directory)
            package = temp / "package.tar.gz"
            package.write_bytes(b"deployment-package")
            package_sha256 = subprocess.check_output(["sha256sum", str(package)], text=True).split()[0]
            package.with_name(f"{package.name}.sha256").write_text(
                f"{package_sha256}  {package.name}\n", encoding="utf-8"
            )
            remote_dir = temp / "remote"
            remote_dir.mkdir()
            result = subprocess.run(
                ["bash", "-c", r'''
set -euo pipefail
source "$1"
PACKAGE_PATH="$2"
REMOTE=fixture.invalid
REMOTE_PACKAGE="$3/a4n-deploy.tar.gz"
PACKAGE_SIZE=18K
SSH_OPTS=()
log_section() { :; }
log_info() { printf '%s\n' "$*"; }
log_ok() { :; }
die() { printf '%s\n' "$*" >&2; exit 1; }
command() {
    if [[ "${1:-}" == '-v' && "${2:-}" == rsync ]]; then
        return 1
    fi
    builtin command "$@"
}
ssh() {
    local command="${!#}"
    bash -c "$command"
}
upload_package
test -f "$3/a4n-deploy.tar.gz"
test ! -e "$3/a4n-deploy.tar.gz.part"
cmp "$2" "$3/a4n-deploy.tar.gz"
''', "test", str(CLOUD.parent / "deploy-cloud/build.sh"), str(package), str(remote_dir)],
                cwd=ROOT, text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("SSH", result.stdout)


if __name__ == "__main__":
    unittest.main()
