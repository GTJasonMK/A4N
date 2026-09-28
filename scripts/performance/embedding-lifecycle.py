#!/usr/bin/env python3
"""Measure real local-model embedding RPCs with an explicitly downloaded test model."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import http.client
import json
import math
import os
from pathlib import Path
import secrets
import shutil
import socket
import statistics
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/tests"))
from run_export_worker_runtime_e2e import service_request, start_process, stop_process


def resources(pid):
    status = dict(line.split(":", 1) for line in (Path("/proc") / str(pid) / "status").read_text().splitlines())
    return {"rss_kib": int(status["VmRSS"].split()[0]), "peak_rss_kib": int(status["VmHWM"].split()[0]),
            "threads": int(status["Threads"])}


def envelope(payload, request_id):
    request = service_request(payload, request_id=request_id)
    request["capability_version"] = "embedding-v5"
    return request


def rpc(connection, token, path, payload, request_id, expected_status="ok"):
    body = json.dumps(envelope(payload, request_id), ensure_ascii=False).encode()
    started = time.perf_counter()
    connection.request("POST", path, body, {"Content-Type": "application/json", "x-a4n-sidecar-token": token})
    response = connection.getresponse()
    raw = response.read()
    elapsed = (time.perf_counter() - started) * 1000
    result = json.loads(raw)
    if response.status != 200 or result.get("status") != expected_status:
        raise RuntimeError(f"{path}: HTTP {response.status}, {result.get('error')}")
    return result["payload" if expected_status == "ok" else "error"], elapsed


def download_model(port, token, model, output):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
    completed = False
    last_print = 0
    try:
        connection.request("POST", "/embedding/model/download/events",
                           json.dumps(envelope({"model": model}, "download-model")),
                           {"Content-Type": "application/json", "x-a4n-sidecar-token": token})
        response = connection.getresponse()
        if response.status != 200:
            raise RuntimeError(f"Model download HTTP {response.status}")
        with (output / "download-events.jsonl").open("w") as log:
            for line in response:
                event = json.loads(line)
                log.write(json.dumps(event, ensure_ascii=False) + "\n")
                log.flush()
                if event["event_kind"] == "failed":
                    raise RuntimeError(f"Model download failed: {event['error']}")
                if event["event_kind"] == "completed":
                    completed = True
                now = time.monotonic()
                if event["event_kind"] == "progress" and now - last_print >= 2:
                    print("Model download", event.get("payload"), flush=True)
                    last_print = now
        if not completed:
            raise RuntimeError("Model download ended without a completed event")
    finally:
        connection.close()


def fixture_texts(count):
    return [f"片段 {index}：雨夜里，林岚走过旧车站，挂钟停在午夜。她比较两封信的日期，寻找时间错位的证据。"
            for index in range(count)]


def verify_file_lifecycle(port, token, model, cache, output):
    repo = cache / ("models--" + model["model_code"].replace("/", "--"))
    assert repo.resolve().is_relative_to(cache.resolve())
    reference = json.loads((output / "vectors-1.json").read_text())
    payload = {"backend": "fastembed", "model": model["name"], "dimension": model["dimension"],
               "batch_size": 32, "texts": fixture_texts(1)}
    client = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
    checks = []

    def rejects_stale_model(label):
        candidates = {path.resolve() for path in repo.rglob("*.onnx")}
        assert len(candidates) == 1, "File checks require an isolated single-ONNX-model cache"
        weights = candidates.pop()
        assert weights.is_relative_to(repo.resolve())
        backup = weights.with_name(weights.name + ".perf-backup")
        invalid = weights.with_name(weights.name + ".perf-invalid")
        assert not backup.exists() and not invalid.exists(), "An earlier fixture needs inspection"
        digest = hashlib.sha256(weights.read_bytes()).hexdigest()
        shutil.copy2(weights, backup)
        try:
            invalid.write_bytes(b"invalid ONNX test fixture")
            # Replace the inode; do not truncate a file an incorrect implementation may still map.
            os.replace(invalid, weights)
            error, _ = rpc(client, token, "/embedding/embed_texts", payload, label + "-invalid", "error")
            assert "初始化 fastembed 模型失败" in error["message"], error
        finally:
            os.replace(backup, weights)
        assert hashlib.sha256(weights.read_bytes()).hexdigest() == digest
        restored, _ = rpc(client, token, "/embedding/embed_texts", payload, label + "-restored")
        assert restored == reference, "Restoring the same weights must restore identical vectors"
        checks.append(label + " invalidates retained weights; failed initialization then recovers")

    try:
        for batch in [1, 8, 32]:
            varied, _ = rpc(client, token, "/embedding/embed_texts",
                            {**payload, "batch_size": batch, "texts": fixture_texts(8)}, f"batch-{batch}")
            assert len(varied["embeddings"]) == 8
            (output / f"vectors-batch-{batch}.json").write_text(json.dumps(varied))
        checks.append("different batch sizes produce fresh responses for comparison")
        for invalid_input in [{"dimension": model["dimension"] + 1}, {"batch_size": 0}, {"model": "not-a-supported-model"}]:
            rpc(client, token, "/embedding/embed_texts", {**payload, **invalid_input}, "invalid-config", "error")
        recovered, _ = rpc(client, token, "/embedding/embed_texts", payload, "after-invalid-config")
        assert recovered == reference
        checks.append("invalid configurations reject without damaging the valid model")

        def embedding_metrics():
            client.request("GET", "/metrics", headers={"x-a4n-sidecar-token": token})
            response = client.getresponse()
            assert response.status == 200
            return json.loads(response.read())["operations"]["/embedding/embed_texts"]

        before_cancel = embedding_metrics()["cancelled_total"]
        pending = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
        try:
            pending.request("POST", "/embedding/embed_texts",
                            json.dumps(envelope({**payload, "texts": fixture_texts(512)}, "cancel-inference"), ensure_ascii=False).encode(),
                            {"Content-Type": "application/json", "x-a4n-sidecar-token": token})
            deadline = time.monotonic() + 5
            while embedding_metrics()["in_flight"] == 0:
                if time.monotonic() >= deadline:
                    raise RuntimeError("The cancellable inference did not enter the service")
                time.sleep(.01)
        finally:
            pending.close()
        recovered, _ = rpc(client, token, "/embedding/embed_texts", payload, "after-cancelled-inference")
        assert recovered == reference
        metrics = embedding_metrics()
        assert metrics["cancelled_total"] > before_cancel and metrics["in_flight"] == 0, metrics
        checks.append("disconnect cancels an in-flight request and the next inference completes identically")

        download_model(port, token, model["name"], output)
        rejects_stale_model("download")
        saved_repo = output / "model-repository-backup"
        shutil.copytree(repo, saved_repo, symlinks=True)
        try:
            rpc(client, token, "/embedding/model/delete", {"model": model["name"]}, "delete-model")
            assert not repo.exists(), "Successful deletion must remove the model repository"
            error, _ = rpc(client, token, "/embedding/embed_texts", payload, "missing-model", "error")
            assert "尚未下载" in error["message"], error
            checks.append("deleted model rejects instead of using resident weights")
        finally:
            if not repo.exists():
                shutil.copytree(saved_repo, repo, symlinks=True)
        rejects_stale_model("delete")
        return checks
    finally:
        client.close()
        (output / "file-lifecycle-checks.json").write_text(json.dumps(checks, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--service", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--report-dir", type=Path, required=True)
    parser.add_argument("--model", default="BGESmallZHV15")
    parser.add_argument("--download", action="store_true", help="explicitly download the test model when it is missing")
    parser.add_argument("--verify-files", action="store_true", help="verify batch changes, cancellation, download/delete and restored test files after measurement")
    parser.add_argument("--requests", type=int, default=9, help="requests per client, including one warmup")
    args = parser.parse_args()
    if args.requests < 3:
        parser.error("at least three requests per client are required")
    service, state, output = args.service.resolve(), args.state_dir.resolve(), args.report_dir.resolve()
    if not service.is_relative_to(ROOT) or any(not p.is_relative_to(ROOT / "data") for p in [state, output]):
        parser.error("service must stay in the worktree; state and reports must stay under its data directory")
    state.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=False)
    state.chmod(0o700)
    output.chmod(0o700)
    cache = state / "models"
    cache.mkdir(exist_ok=True)
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    token = secrets.token_urlsafe(32)
    ort = Path.home() / ".local/share/a4n/onnxruntime" / (ROOT / "tools/onnxruntime/VERSION").read_text().strip()
    env = {key: value for key, value in os.environ.items() if not key.startswith("A4N_")}
    env.update({"A4N_EMBEDDING_SERVICE_ADDR": f"127.0.0.1:{port}", "A4N_SIDECAR_AUTH_TOKEN": token,
                "A4N_EMBEDDING_SERVICE_LOG": "warn", "A4N_EMBEDDING_CACHE_DIR": str(cache),
                "A4N_ONNXRUNTIME_ROOT": str(ort), "ORT_DYLIB_PATH": str(ort / "lib/libonnxruntime.so")})
    env["LD_LIBRARY_PATH"] = os.pathsep.join(value for value in [str(ort / "lib"), env.get("LD_LIBRARY_PATH")] if value)
    process = None
    report = {"success": False, "binary_sha256": hashlib.sha256(service.read_bytes()).hexdigest(), "metrics": []}
    try:
        process = start_process("embedding", [str(service)], env=env, cwd=state, log_path=output / "service.log")
        deadline = time.monotonic() + 15
        while True:
            if process.process.poll() is not None:
                raise RuntimeError("Embedding service exited; see service.log")
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
            try:
                connection.request("GET", "/healthz", headers={"x-a4n-sidecar-token": token})
                response = connection.getresponse()
                if response.status == 200:
                    response.read()
                    break
            except (OSError, http.client.HTTPException):
                pass
            finally:
                connection.close()
            if time.monotonic() >= deadline:
                raise RuntimeError("Embedding service readiness timed out")
            time.sleep(.05)
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
        try:
            models, _ = rpc(connection, token, "/embedding/models", None, "models-before")
            model = next(item for item in models if item["name"] == args.model)
            report["model"] = model
            if not model["downloaded"]:
                if not args.download:
                    raise RuntimeError("The test model is missing; use the explicit --download setup once")
                download_model(port, token, args.model, output)
                models, _ = rpc(connection, token, "/embedding/models", None, "models-after")
                model = next(item for item in models if item["name"] == args.model)
                assert model["downloaded"], model
                report["model"] = model
        finally:
            connection.close()
        report["before_load"] = resources(process.process.pid)
        for text_count in [1, 32]:
            texts = fixture_texts(text_count)
            for concurrency in [1, 4]:
                def series(client_number):
                    client = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
                    samples = []
                    try:
                        for index in range(args.requests):
                            value, elapsed = rpc(client, token, "/embedding/embed_texts",
                                {"backend": "fastembed", "model": args.model, "dimension": model["dimension"],
                                 "batch_size": 32, "texts": texts}, f"embed-{text_count}-{client_number}-{index}")
                            assert value["identity"]["dimension"] == model["dimension"], value["identity"]
                            assert len(value["embeddings"]) == text_count
                            assert all(len(vector) == model["dimension"] and all(math.isfinite(x) for x in vector)
                                       for vector in value["embeddings"])
                            digest = hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()
                            samples.append({"elapsed_ms": elapsed, "sha256": digest, "warmup": index == 0})
                            if client_number == 0 and index == 0:
                                (output / f"vectors-{text_count}.json").write_text(json.dumps(value))
                    finally:
                        client.close()
                    return samples
                with ThreadPoolExecutor(max_workers=concurrency) as pool:
                    samples = [item for batch in pool.map(series, range(concurrency)) for item in batch]
                assert len({item["sha256"] for item in samples}) == 1, "The baseline itself is not bitwise deterministic"
                times = sorted(item["elapsed_ms"] for item in samples if not item["warmup"])
                metric = {"texts": text_count, "concurrency": concurrency, "requests": len(times),
                          "p50_ms": statistics.median(times), "p95_ms": times[math.ceil(len(times) * .95) - 1],
                          "resources": resources(process.process.pid), "samples": samples}
                report["metrics"].append(metric)
                (output / "report.json").write_text(json.dumps(report, indent=2))
                print(text_count, "texts; clients", concurrency, "p50", round(metric["p50_ms"], 2),
                      "p95", round(metric["p95_ms"], 2), "ms;", metric["resources"], flush=True)
        if args.verify_files:
            report["file_lifecycle"] = verify_file_lifecycle(port, token, model, cache, output)
        time.sleep(.5)
        stat = Path("/proc") / str(process.process.pid) / "stat"
        def cpu_ticks():
            fields = stat.read_text().split(") ", 1)[1].split()
            return int(fields[11]) + int(fields[12])
        ticks = cpu_ticks()
        started = time.perf_counter()
        time.sleep(1)
        report["settled_idle_cpu_percent"] = (cpu_ticks() - ticks) / os.sysconf("SC_CLK_TCK") / (time.perf_counter() - started) * 100
        report["success"] = True
    except BaseException as error:
        report["error"] = str(error)
        raise
    finally:
        stop_process(process)
        (output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
