"""Actual CLI/Worker regression checks, including 100 durable summary calls."""
import copy
import argparse
import csv
from datetime import datetime, timezone
import hashlib
import io
import json
import os
import re
from pathlib import Path
import signal
import subprocess
import sys
import time
import traceback
from types import SimpleNamespace
from urllib.parse import urlparse

repo = Path(__file__).resolve().parents[2]
from run_cli_contract_e2e import Cli, checked, runtime_snapshot, secure_json, start_provider
from run_agent_cli_e2e import thread_snapshot, run_view

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--a4n", type=Path, default=repo / "target/debug/a4n")
parser.add_argument("--server", required=True)
parser.add_argument("--username", required=True)
parser.add_argument("--password-env", default="A4N_PASSWORD")
parser.add_argument("--report-dir", type=Path, required=True)
parser.add_argument("--fault-database-url-env", required=True)
parser.add_argument("--postgres-container", default="a4n-postgres")
parser.add_argument("--provider-url")
parser.add_argument("--shutdown-worker-binary", type=Path, help="Opt in to SIGTERM of this local fixture's Worker binary")
args = parser.parse_args()
database_url = os.environ[args.fault_database_url_env]
uri = urlparse(database_url)
if not re.fullmatch(r"/a4n_agent_[0-9a-f]+", uri.path):
    parser.error("fault database must be an isolated a4n_agent_<hex> database")
if args.shutdown_worker_binary and urlparse(args.server).hostname not in {"127.0.0.1", "localhost", "::1"}:
    parser.error("Worker SIGTERM requires a loopback fixture Server")
evidence = args.report_dir.resolve() / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
evidence.mkdir(parents=True, exist_ok=True)
config_path = evidence / "cli.json"
environment = os.environ.copy()
password = environment.pop(args.password_env)
environment.pop("A4N_ACCESS_TOKEN", None)
cli = Cli(str(args.a4n.resolve()), args.server, config_path, evidence / "context.json", 30, environment)
settings = SimpleNamespace(provider_url=args.provider_url, provider_host="127.0.0.1", provider_port=0, provider_script=repo / "scripts/tests/mock_openai_provider.py", provider_start_timeout=10, embedding_dimension=384)
provider, log_path = start_provider(settings, evidence, "edges")
normal = runtime_snapshot(provider.base_url, embedding_dimension=384)
report = {"success": False, "checks": [], "provider_log": str(log_path)}

def command(parts):
    return checked(cli, parts, parts[0])[0]["data"]

def use_config(value):
    path = evidence / "runtime.json"
    secure_json(path, value)
    command(["config", "runtime", "set", "--input", "@" + str(path)])

def new_thread():
    return command(["agent", "thread", "create", "--kind", "main", "--capability", "main"])["id"]

def send(thread, task, budget=None):
    parts = ["agent", "chat", thread, "A4N_RELIABILITY " + json.dumps(task, ensure_ascii=False)]
    if budget: parts += ["--budget", json.dumps(budget)]
    return command(parts)["run_id"]

def wait(run_id, status, reason=None, timeout=60):
    deadline = time.monotonic() + timeout
    progress = -1
    while time.monotonic() < deadline:
        if status == "recovery_required":
            response = cli.run(["agent", "run-show", run_id])
            # This one check deliberately restarts the Worker serving reads.
            if response["exit_code"] == 7:
                time.sleep(0.15)
                continue
            assert response["exit_code"] == 0, response["stderr"]
            run = response["stdout_documents"][-1]["data"]
        else:
            run = run_view(cli, run_id)
        if run["model_calls_used"] // 10 != progress:
            progress = run["model_calls_used"] // 10
            print(json.dumps({"run_id": run_id, "status": run["status"], "model_calls": run["model_calls_used"]}), flush=True)
        if run["status"] == status and (reason is None or run["waiting_reason"] == reason): return run
        assert run["status"] not in {"failed", "cancelled", "completed", "recovery_required"}, run
        time.sleep(0.15)
    raise AssertionError({"timeout": run_id, "last": run})

def save(name, thread):
    value = thread_snapshot(cli, thread)
    secure_json(evidence / (name + ".json"), value)
    return value

try:
    command(["health"])
    command(["capabilities", "check"])
    checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")
    use_config(normal)
    project = command(["project", "create", "--title", "Agent 边界验收", "--project-type", "novel_long", "--use-context"])
    thread = new_thread()
    run_id = send(thread, {"mode": "unknown_tool", "project_id": project["id"]})
    wait(run_id, "completed")
    snapshot = save("unknown-tool-and-reused-id", thread)
    tools = [row for row in snapshot["executions"] if row["kind"] == "tool_call"]
    assert [row["status"] for row in tools] == ["failed", "succeeded"], tools
    assert len({row["id"] for row in tools}) == 2
    report["checks"].append("unknown tool becomes an observation; repeated provider IDs do not collide")

    thread = new_thread()
    run_id = send(thread, {"mode": "question"})
    wait(run_id, "waiting", "user")
    before = save("question-wait", thread)
    answer = command(["agent", "chat", thread, "42"])
    assert answer["run_id"] == run_id
    wait(run_id, "completed")
    after = save("question-answer", thread)
    questions = [message for message in after["messages"] if message["metadata"] and message["metadata"].get("question")]
    assert len(questions) == 1 and any(message["content"] == "已收到42" for message in after["messages"])
    report["checks"].append("question and answer resume the original Run exactly once")

    long_config = copy.deepcopy(normal)
    for llm in [long_config["llm"], *long_config["llm_configs"]]:
        llm["max_context_tokens"] = 24000
        llm["max_output_tokens"] = 1024
    use_config(long_config)
    thread = new_thread()
    csv_data = io.StringIO()
    writer = csv.writer(csv_data, lineterminator="\n")
    timestamp = datetime.now(timezone.utc).replace(tzinfo=None).isoformat()
    texts = ["长期约束：地点名是白塔。最终按最新指令作答，保留这个原始事实。"] + [f"Archived evidence {index}: " + "z" * 7600 for index in range(100)]
    for index, text in enumerate(texts):
        writer.writerow([int(thread), "user" if index == 0 else "assistant", "user" if index == 0 else "agent", hashlib.sha256(text.encode()).hexdigest(), text, "complete", timestamp, timestamp])
    # COPY treats all fixture identities/content as data, not SQL interpolation.
    sql = "COPY agent_message(thread_id,role,sender_kind,content_sha256,content,status,created_at,updated_at) FROM STDIN WITH (FORMAT csv);\n" + csv_data.getvalue() + "\\.\n"
    seeded = subprocess.run(["docker", "exec", "-i", args.postgres_container, "psql", "-v", "ON_ERROR_STOP=1", "-U", uri.username, "-d", uri.path[1:]], input=sql, text=True, capture_output=True)
    assert seeded.returncode == 0, seeded.stderr
    run_id = send(thread, {"mode": "summary_final", "instruction": "仅输出历史约定的地点名"}, {"max_steps": 256, "max_model_calls": 256})
    run = wait(run_id, "completed", timeout=120)
    snapshot = save("one-hundred-summaries", thread)
    summaries = [row for row in snapshot["executions"] if row["kind"] == "model_call" and row["result_summary"] == "Agent context summary"]
    assert len(summaries) >= 100, len(summaries)
    assert all(row["status"] == "succeeded" for row in summaries)
    assert run["model_calls_used"] == len(summaries) + 1 and run["usage_complete"]
    assert any(message["content"] == "白塔" for message in snapshot["messages"] if message["role"] == "assistant")
    report["checks"].append(f"{len(summaries)} durable summary executions through the mock Provider preserve the original constraint, budget receipts and completion")

    slow = copy.deepcopy(normal)
    for llm in [slow["llm"], *slow["llm_configs"]]: llm["model"] = "a4n-llm-lifecycle-slow"
    use_config(slow)
    slow_thread = new_thread()
    slow_run = send(slow_thread, {"mode": "plain"})
    wait(slow_run, "running")
    use_config(normal)
    fast_thread = new_thread()
    fast_run = send(fast_thread, {"mode": "plain"})
    wait(fast_run, "completed", timeout=4)
    assert run_view(cli, slow_run)["status"] == "running"
    command(["agent", "run", slow_run, "cancel", "--thread-id", slow_thread])
    assert run_view(cli, slow_run)["status"] == "cancelled"
    assert not any(row["status"] == "streaming" for row in save("concurrent-cancel", slow_thread)["messages"])
    report["checks"].append("a long model stream does not block another Run; cancellation closes its projection")

    use_config(slow)
    thread = new_thread()
    run_id = send(thread, {"mode": "plain"})
    wait(run_id, "running")
    command(["agent", "run", run_id, "pause", "--thread-id", thread])
    assert run_view(cli, run_id)["status"] == "paused"
    command(["agent", "run", run_id, "resume", "--thread-id", thread])
    resumed = wait(run_id, "completed")
    assert resumed["model_calls_used"] == 2 and resumed["tokens_reserved"] > 0
    save("pause-resume", thread)
    report["checks"].append("pause/resume reuses the Run and conservatively retains unknown interrupted-call usage")

    if args.shutdown_worker_binary:
        thread = new_thread()
        run_id = send(thread, {"mode": "plain"})
        running = wait(run_id, "running")
        deadline = time.monotonic() + 30
        while not any(message["status"] == "streaming" for message in thread_snapshot(cli, thread)["messages"]):
            assert time.monotonic() < deadline, "Worker never opened its model stream"
            time.sleep(0.05)
        owner = running["lease_owner"]
        assert owner.startswith("a4n-agent:")
        pid = int(owner.split(":")[1])
        assert Path(f"/proc/{pid}/exe").resolve() == args.shutdown_worker_binary.resolve(), "Worker binary does not match the opted-in fixture"
        environment = dict(item.split(b"=", 1) for item in Path(f"/proc/{pid}/environ").read_bytes().split(b"\0") if b"=" in item)
        assert environment.get(b"A4N_DATABASE_URL") == database_url.encode(), "Worker database does not match this fixture"
        os.kill(pid, signal.SIGTERM)
        recovered = wait(run_id, "recovery_required")
        assert recovered["lease_owner"] is None and recovered["tokens_reserved"] > 0
        # The old Worker can persist recovery before its HTTP listener closes.
        # Only this deliberate restart permits transient transport failures.
        deadline = time.monotonic() + 30
        report["shutdown_snapshot_retries"] = 0
        while True:
            response = cli.run(["agent", "thread", "show", thread])
            if response["exit_code"] == 0:
                snapshot = response["stdout_documents"][-1]["data"]
                secure_json(evidence / "worker-shutdown.json", snapshot)
                break
            assert response["exit_code"] == 7 and time.monotonic() < deadline, response["stderr"]
            report["shutdown_snapshot_retries"] += 1
            time.sleep(0.15)
        report["checks"].append("Worker SIGTERM persists recovery-required and clears the lease")
    report["shutdown_verified"] = bool(args.shutdown_worker_binary)
    report["success"] = True
except Exception as error:
    report["error"] = f"{type(error).__name__}: {error}"
    report["traceback"] = traceback.format_exc()
finally:
    provider.stop()
    config_path.unlink(missing_ok=True)
    (evidence / "runtime.json").unlink(missing_ok=True)
    secure_json(evidence / "report.json", report)
    secure_json(evidence.parent / "report.json", report)
print(json.dumps(report, ensure_ascii=False), flush=True)
raise SystemExit(0 if report["success"] else 1)
