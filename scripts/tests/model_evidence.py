"""Content-bound offline model acceptance evidence; empty/skipped/stale is failure."""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
INPUT_ROOTS = ("crates/", "prompts/", "scripts/", "frontend/src/", "frontend/e2e/", "frontend/public/", ".github/", ".githooks/", ".cargo/", "tools/onnxruntime/")
INPUT_FILES = {"Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml", ".gitignore", "AGENTS.md"}


class EvidenceError(ValueError):
    pass


def digest_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def input_snapshot(root: Path = ROOT) -> dict[str, Any]:
    names = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=root).decode().split("\0")
    files = {}
    for name in sorted(set(names)):
        frontend_config=name.startswith("frontend/") and name.count("/")==1 and Path(name).suffix in {".json",".ts",".js",".mjs",".cjs",".html",".toml",".yaml",".yml"}
        if not name or not (name in INPUT_FILES or name.startswith(INPUT_ROOTS) or frontend_config):
            continue
        path = root / name
        if not path.exists() and not path.is_symlink():
            continue
        if not path.is_file() or path.is_symlink():
            raise EvidenceError(f"测试输入缺失或为符号链接：{name}")
        files[name] = digest_file(path)
    if not files:
        raise EvidenceError("测试输入清单为空")
    build = {key: os.environ.get(key, "") for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET", "CARGO_TARGET_DIR", "CARGO_INCREMENTAL", "CARGO_PROFILE_DEV_OPT_LEVEL", "CARGO_PROFILE_TEST_OPT_LEVEL", "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG")}
    value = {"files": files, "build_environment": build}
    return {**value, "sha256": hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()}


def seal_artifact(path: Path, report_dir: Path) -> dict[str, str]:
    path = path.resolve(strict=True)
    if not path.is_relative_to(report_dir.resolve()):
        raise EvidenceError("证据文件必须位于本次报告目录")
    return {"path": str(path.relative_to(report_dir.resolve())), "sha256": digest_file(path)}


def verify_artifact(receipt: dict[str, str], report_dir: Path) -> Path:
    name = receipt.get("path")
    if not isinstance(name, str) or not name or Path(name).is_absolute() or ".." in Path(name).parts:
        raise EvidenceError("无效证据路径")
    path = report_dir / name
    if path.is_symlink() or not path.resolve().is_relative_to(report_dir.resolve()) or not path.is_file():
        raise EvidenceError(f"证据文件缺失或越界：{name}")
    if digest_file(path) != receipt.get("sha256"):
        raise EvidenceError(f"证据内容已改变：{name}")
    return path


def verify_report(path: Path, *, root: Path = ROOT, require_full: bool = True) -> dict[str, Any]:
    report = json.loads(path.read_text())
    if report.get("schema_version") != 2 or report.get("success") is not True:
        raise EvidenceError("报告没有成功完成")
    if require_full and report.get("profile") != "model-full":
        raise EvidenceError("合并必须使用 model-full 报告")
    current = input_snapshot(root)
    if report.get("inputs") != current or report.get("finished_inputs_sha256") != current["sha256"]:
        raise EvidenceError("源码、提示词、测试、素材、配置、锁文件或构建选项已改变，旧报告失效")
    binaries = report.get("binaries")
    if not isinstance(binaries, dict) or not binaries:
        raise EvidenceError("报告缺少实际测试二进制")
    for binary, expected in {**binaries,**report.get("native_artifacts",{})}.items():
        candidate = Path(binary)
        if not candidate.is_file() or digest_file(candidate) != expected:
            raise EvidenceError(f"测试二进制缺失或已改变：{binary}")
    expected = report.get("required_cases")
    cases = report.get("cases")
    if not isinstance(expected, list) or not expected or len(expected) != len(set(expected)) or not isinstance(cases, list):
        raise EvidenceError("必测项或执行结果为空/重复")
    observed = {}
    checked=set()
    for case in cases:
        case_id = case.get("case_id")
        if not isinstance(case_id, str) or case_id in observed or case.get("status") != "passed" or case.get("assertions", 0) <= 0:
            raise EvidenceError(f"用例未通过、无断言或重复：{case_id}")
        if not case.get("evidence"):
            raise EvidenceError(f"用例缺少执行证据：{case_id}")
        if "actual" in case or "expected" in case:
            if "actual" not in case or "expected" not in case or json.dumps(case["actual"],sort_keys=True)!=json.dumps(case["expected"],sort_keys=True):
                raise EvidenceError(f"用例实际结果与预期不符：{case_id}")
        for receipt in case["evidence"]:
            key=(receipt.get("path"),receipt.get("sha256"))
            if key not in checked:
                verify_artifact(receipt, path.parent);checked.add(key)
        observed[case_id] = case
    if set(observed) != set(expected):
        raise EvidenceError(f"执行证据覆盖不完整：缺少 {sorted(set(expected)-set(observed))}，多出 {sorted(set(observed)-set(expected))}")
    from model_requirements import required_case_ids
    from check_model_call_coverage import inspect,MANIFEST
    if not report.get("contracts") or not report.get("inventory"):raise EvidenceError("缺少生产合同或 AST 扫描证据")
    corpus=json.loads(verify_artifact(report["contracts"],path.parent).read_text())
    inventory=json.loads(verify_artifact(report["inventory"],path.parent).read_text())
    inspect(root=root,ast=inventory)
    required=required_case_ids(json.loads((root/MANIFEST).read_text()),inventory,corpus,report["profile"])
    if set(expected)!=required:raise EvidenceError("报告声明的必测项与当前业务合同/能力目录不一致")
    suites = report.get("suites")
    if not isinstance(suites, dict) or not suites or any(row.get("status") != "passed" or row.get("executed", 0) <= 0 for row in suites.values()):
        raise EvidenceError("测试套件未执行、跳过、超时或失败")
    if require_full:
        creation = report.get("creation")
        if not isinstance(creation, dict):
            raise EvidenceError("缺少完整 CLI/浏览器流程验证")
        creation_path = verify_artifact(creation, path.parent)
        creation_report = json.loads(creation_path.read_text())
        if creation_report.get("success") is not True or not creation_report.get("source", {}).get("matches_current_worktree"):
            raise EvidenceError("完整创作流程没有在相同版本通过")
        if report.get("build_features")!={"rust_tests":"default","creation":"a4n-server/embedded-capabilities"}:raise EvidenceError("完整验收构建 feature 记录不一致")
        if not report.get("native_artifacts"):raise EvidenceError("完整验收缺少实际媒体工具/运行库指纹")
        tree=subprocess.check_output(["git","rev-parse","HEAD^{tree}"],cwd=root,text=True).strip()
        if report.get("tested_tree")!=tree:raise EvidenceError("完整验收内容树已变化")
        if subprocess.check_output(["git","status","--porcelain"],cwd=root).strip():raise EvidenceError("完整验收的最终工作区必须干净")
    return {"success": True, "profile": report["profile"], "cases": len(cases), "inputs_sha256": current["sha256"], "live_provider_tested": False}
