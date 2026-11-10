#!/usr/bin/env python3
"""Exercise independent media stages and their lifecycle through a real CLI/Host."""

import base64
import copy
import io
import json
import tempfile
import time
import traceback
import uuid
import zipfile
from pathlib import Path
from urllib.request import urlopen

from mock_openai.fixtures.story import chapter_body_response
from mock_openai.fixtures.reverse import message_text
from mock_openai.protocol import PNG_1X1_BASE64
from mock_openai.prompt_parsing import current_task_prompt_name, json_task_payload
from mock_openai.contract_guard import fixture_prompt_parts
from run_cli_contract_e2e import checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--embedding-dimension", type=int, default=384)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "projects": []}
    try:
        with tempfile.TemporaryDirectory(prefix="media-authoring-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            mock_runtime = runtime_snapshot(
                args.provider_url,
                embedding_dimension=args.embedding_dimension,
            )
            report["runtime_config_snapshot"] = mock_runtime
            secure_json(
                work / "runtime.json",
                mock_runtime,
            )
            checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "configure mock provider")
            described = set()

            def call(name, value, *, raw=False, destructive=False, reject=None):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                parts = ["call", name, "--args", "-"]
                if destructive:
                    parts.append("--allow-destructive")
                encoded = json.dumps(value if raw else {"input": value}, ensure_ascii=False)
                if reject:
                    result = cli.run(parts, encoded)
                    error = command_error(result)
                    assert result["exit_code"] != 0 and reject in str(error), (name, result)
                    return error
                return checked(cli, parts, name, encoded)[0]["data"]

            def passed(message):
                report["checks"].append(message)
                secure_json(args.report, report)

            def llm_calls():
                return [row for row in provider_log_entries(args.provider_log) if isinstance(row.get("body", {}).get("messages"), list)]

            def workspace(project_id):
                return call("get_media_authoring_workspace", {"project_id": project_id, "chapter_id": None})

            def latest(project_id, kind, status):
                return next(view["record"] for view in workspace(project_id)["documents"] if view["record"]["kind"] == kind and view["record"]["status"] == status)

            def adopt(record):
                return call("adopt_media_authoring_document", {"project_id": record["project_id"], "record_id": record["id"], "expected_revision": record["revision"]})

            def save(record, document):
                return call("save_media_authoring_document", {"project_id": record["project_id"], "chapter_id": record["source_chapter_id"], "kind": record["kind"], "record_id": record["id"], "expected_revision": record["revision"], "document": document})

            def generate(project_id, base=None, reference=None):
                source_chapter_id = workspace(project_id)["chapters"][0]["source"]["source_chapter_id"]
                return call("generate_media_adaptation_plan", {
                    "project_id": project_id, "chapter_id": source_chapter_id, "task_id": str(uuid.uuid4()),
                    "instruction": "保留原作因果，以本媒介独立改编", "video_reference": reference,
                    "clear_video_reference": False, "record_id": base["id"] if base else None,
                    "expected_revision": base["revision"] if base else None, "resume_from_checkpoint": True,
                })

            root = checked(cli, ["project", "create", "--title", "派生媒体独立创作验收", "--project-type", "novel_long", "--use-context"], "create root")[0]["data"]
            chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "旧车站", "--content", chapter_body_response()], "import source")[0]["data"]
            root_id, chapter_id = root["id"], chapter["id"]
            report.update(root_id=root_id, chapter_id=chapter_id)
            for name in ("林岚", "周衡"):
                profile = call("create_profile", {"project_id": root_id, "name": name, "base_attributes": {"explicit_attributes": {}, "implicit_attributes": {}, "social_attributes": {}}})
                profile_id = profile.get("profile", {}).get("id")
                assert isinstance(profile_id, int) and profile_id > 0, profile
                call("upload_portrait", {
                    "project_id": root_id,
                    "profile_id": profile_id,
                    "file_name": f"media-authoring-{profile_id}.png",
                    "mime_type": "image/png",
                    "data_base64": PNG_1X1_BASE64,
                    "portrait_type": "face",
                    "prompt": f"{name} 的漫画参考立绘",
                    "set_primary": True,
                })
            call("create_scene_profile", {"project_id": root_id, "profile": {"name": "旧车站通道", "scene_type": "room", "importance": "primary", "base_description": "封闭通道",
                "visual_description": "木门、挂钟与清晰的通道", "spatial_layout": "通道通向机房", "atmosphere": "安静", "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": "旧车站通道"}})

            def derive(kind, title=None):
                project = call("create_derived_project", {"root_project_id": root_id, "derived_kind": kind, "title": title or "独立" + kind, "description": "isolated stage fixture", "start_chapter": 1, "end_chapter": 1})
                report["projects"].append(project["id"])
                return project["id"]

            comic, game = derive("comic"), derive("galgame")
            assert call("list_video_adaptation_references", {"project_id": comic, "chapter_id": chapter_id}) == []

            def manga_run(project_id=comic, stop_after=None, page_number=None, run_stage=None, expected_error=None, resume=False):
                pipeline_input = {
                    "meta": {"start_request_id": str(uuid.uuid4())},
                    "project_id": project_id,
                    "chapter_number": 1,
                    "image_size": {"width": 1024, "height": 1024},
                }
                if stop_after is not None:
                    pipeline_input["stop_after"] = stop_after
                if page_number is not None:
                    pipeline_input["page_number"] = page_number
                if run_stage is not None:
                    pipeline_input["run_stage"] = run_stage
                handle = call("resume_manga_pipeline" if resume else "start_manga_pipeline", pipeline_input)
                if expected_error is None:
                    checked(
                        cli,
                        ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])],
                        "manga task watch",
                    )
                deadline = time.monotonic() + 60
                while True:
                    task = checked(
                        cli,
                        ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])],
                        "manga task",
                    )[0]["data"]
                    if task["handle"]["status"] in {"completed", "failed", "cancelled", "interrupted"}:
                        break
                    assert time.monotonic() < deadline, task
                    time.sleep(0.1)
                assert task["handle"]["status"] == ("failed" if expected_error else "completed"), task
                if expected_error:
                    assert expected_error in json.dumps(task, ensure_ascii=False), task
                return task

            def preprocessing_model(name, max_concurrent=4, provider=None):
                snapshot = copy.deepcopy(mock_runtime)
                snapshot["llm"]["model"] = name
                snapshot["llm"]["max_output_tokens"] = 20000
                snapshot["llm"]["max_concurrent"] = max_concurrent
                if provider is not None:
                    snapshot["llm"]["provider"] = provider
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "preprocessing model")

            preprocessing_names = ["manga_preprocessing_" + step for step in ("facts", "speech", "visuals", "synthesis")]

            def new_preprocessing_calls(offset):
                result = []
                for row in llm_calls()[offset:]:
                    body = row["body"]
                    text = "\n".join(message.get("content", "") for message in body["messages"] if isinstance(message.get("content"), str))
                    name = current_task_prompt_name(text)
                    if name in preprocessing_names:
                        result.append((name, body))
                return result

            preprocessing_model("a4n-preprocessing-parallel")
            preview = call("preview_start_manga_pipeline_request", {"project_id": comic, "chapter_number": 1, "run_stage": "preprocessing"})
            assert [step["prompt_name"] for step in preview["steps"]] == preprocessing_names
            assert [step["state"] for step in preview["steps"]] == ["ready", "waiting_dependencies", "waiting_dependencies", "waiting_dependencies"]
            assert preview["steps"][0]["request"]["max_tokens"] == 20000
            before = len(llm_calls())
            manga_run(comic, run_stage="preprocessing")
            initial_calls = new_preprocessing_calls(before)
            assert len(initial_calls) == 4, [name for name, _ in initial_calls]
            assert initial_calls[0][0] == preprocessing_names[0] and initial_calls[-1][0] == preprocessing_names[-1]
            assert {name for name, _ in initial_calls[1:3]} == set(preprocessing_names[1:3])
            assert all(body.get("max_tokens") == 20000 for _, body in initial_calls)
            passed("漫画预处理恰好四次调用，两个提取分支通过 Provider 并发屏障，实际请求和预览均保留 20000 输出额度")

            recover_project = derive("comic", "漫画预处理局部续跑")
            preprocessing_model("a4n-preprocessing-fail-speech")
            before = len(llm_calls())
            failed = manga_run(recover_project, run_stage="preprocessing", expected_error="输出纠正仍未完成")
            failed_calls = new_preprocessing_calls(before)
            assert [name for name, _ in failed_calls].count(preprocessing_names[1]) == 2
            assert not any(name == preprocessing_names[3] for name, _ in failed_calls)
            assert not any(item["stage"] == "preprocessing" for item in call("list_manga_pipeline_artifacts", {"project_id": recover_project, "chapter_id": chapter_id}))
            preprocessing_model("a4n-virtual-text")

            def planning_project(title, count=2):
                preprocessing_model("a4n-virtual-text")
                project = derive("comic", title)
                manga_run(project, run_stage="preprocessing")
                record = generate(project)
                if count != record["document"]["page_count"]:
                    document = copy.deepcopy(record["document"])
                    document["page_count"] = count
                    record = save(record, document)
                adopt(record)
                return project

            def planning_calls(offset):
                result = []
                for row in llm_calls()[offset:]:
                    body = row["body"]
                    text = fixture_prompt_parts(body["messages"])[1]
                    name = current_task_prompt_name(text)
                    if name in {"manga_page_outline", "manga_page_content"}:
                        result.append((name, body, json_task_payload(text)["output_contract"].get("page_number")))
                return result

            parallel_project = planning_project("分页逐页并发")
            preprocessing_model("a4n-page-planning-parallel")
            preview = call("preview_start_manga_pipeline_request", {"project_id": parallel_project, "chapter_number": 1, "run_stage": "page_slots"})
            assert [step["state"] for step in preview["steps"]] == ["ready", "waiting_dependencies", "waiting_dependencies"]
            assert [step["prompt_name"] for step in preview["steps"]] == ["manga_page_outline", "manga_page_content", "manga_page_content"]
            assert preview["steps"][0]["request"]["max_tokens"] == 20000
            before = len(llm_calls())
            manga_run(parallel_project, run_stage="page_slots")
            observed = planning_calls(before)
            assert len(observed) == 3 and observed[0][0] == "manga_page_outline", observed
            assert sorted(page for name, _, page in observed if name == "manga_page_content") == [1, 2]
            assert all(body["max_tokens"] == 20000 for _, body, _ in observed)
            with urlopen(args.provider_url.removesuffix("/v1") + "/fixtures/page-planning", timeout=5) as response:
                assert json.load(response)["a4n-page-planning-parallel"]["max_active"] == 2
            passed("分页按已采用页数执行 1 次轻量页纲与 N 次单页内容；真实 Provider 屏障证明两页并发，预算保持 20000")

            partial_project = planning_project("分页单页失败局部续跑", count=4)
            preprocessing_model("a4n-page-planning-fail-content")
            before = len(llm_calls())
            manga_run(partial_project, run_stage="page_slots", expected_error="reaction")
            observed = planning_calls(before)
            assert len(observed) == 4, [(name, page) for name, _, page in observed]
            assert sorted(page for name, _, page in observed if name == "manga_page_content") == [1, 2, 2]
            assert not any(item["stage"] in {"page_slots", "page_content"} for item in call("list_manga_pipeline_artifacts", {"project_id": partial_project, "chapter_id": chapter_id}))
            preprocessing_model("a4n-virtual-text")
            preview = call("preview_resume_manga_pipeline_request", {"project_id": partial_project, "chapter_number": 1, "run_stage": "page_slots"})
            assert [step["state"] for step in preview["steps"]] == ["reused", "reused", "ready", "ready", "ready"]
            before = len(llm_calls())
            manga_run(partial_project, run_stage="page_slots", resume=True)
            observed = planning_calls(before)
            assert sorted((name, page) for name, _, page in observed) == [("manga_page_content", 2), ("manga_page_content", 3), ("manga_page_content", 4)]
            complete = next(item for item in call("list_manga_pipeline_artifacts", {"project_id": partial_project, "chapter_id": chapter_id}) if item["stage"] == "page_slots")
            assert len(complete["payload"]["pages"]) == 4
            for previous, following in zip(complete["payload"]["pages"], complete["payload"]["pages"][1:]):
                assert previous["content"]["exit_state"] == following["content"]["entry_state"]
            passed("第 2 页缺字段仅纠正一次；失败后停止派发后续页，不发布残缺产物；续跑只补第 2–4 页")

            correction_project = planning_project("分页单页纠正")
            preprocessing_model("a4n-page-planning-recover-content")
            before = len(llm_calls())
            manga_run(correction_project, run_stage="page_slots")
            observed = planning_calls(before)
            assert len(observed) == 4 and [page for name, _, page in observed if name == "manga_page_content"].count(2) == 2
            passed("单页缺失 nullable 字段经过唯一纠正后完成，不重调页纲和其他页面")

            eof_project = planning_project("分页断流恢复")
            preprocessing_model("a4n-page-planning-eof")
            manga_run(eof_project, run_stage="page_slots", expected_error="LLM")
            assert not any(item["stage"] == "page_slots" for item in call("list_manga_pipeline_artifacts", {"project_id": eof_project, "chapter_id": chapter_id}))
            preprocessing_model("a4n-virtual-text")
            before = len(llm_calls())
            manga_run(eof_project, run_stage="page_slots", resume=True)
            assert [(name, page) for name, _, page in planning_calls(before)] == [("manga_page_content", 2)]
            passed("单页 Provider 断流显式失败；恢复仅调用失败页，不重调成功页和页纲")

            serial_project = planning_project("分页遵循模型并发限制")
            preprocessing_model("a4n-page-planning-serial", max_concurrent=1)
            manga_run(serial_project, run_stage="page_slots")
            with urlopen(args.provider_url.removesuffix("/v1") + "/fixtures/page-planning", timeout=5) as response:
                assert json.load(response)["a4n-page-planning-serial"]["max_active"] == 1
            passed("模型配置 max_concurrent=1 时，逐页规划真实请求串行执行")
            preprocessing_model("a4n-virtual-text")

            deletion_project = planning_project("分页删除与手工产物续跑回归")
            saved_slots = next(item for item in call("list_manga_pipeline_artifacts", {"project_id": parallel_project, "chapter_id": chapter_id}) if item["stage"] == "page_slots")
            manual_slots = copy.deepcopy(saved_slots["payload"])
            manual_slots["pages"][0]["content"]["summary"] += "人工保留的页面修改。"
            call("save_manga_pipeline_artifact", {
                "project_id": deletion_project, "chapter_id": chapter_id, "stage": "page_slots",
                "page_number": None, "expected_prompt_updated_at": None, "payload": manual_slots,
            })
            preview = call("preview_start_manga_pipeline_request", {"project_id": deletion_project, "chapter_number": 1, "stop_after": "page_adaptation"})
            assert preview["steps"][0]["key"] == "page_slots" and preview["steps"][0]["state"] == "reused", preview
            assert not any(step.get("prompt_name") in {"manga_page_outline", "manga_page_content"} for step in preview["steps"]), preview
            before = len(llm_calls())
            manga_run(deletion_project, stop_after="page_adaptation")
            assert len(llm_calls()) - before == 2
            assert call("list_manga_page_prompts", {"project_id": deletion_project, "chapter_id": chapter_id})[0]["page_content"]["summary"].endswith("人工保留的页面修改。")
            passed("手工分页的请求指纹不同仍复用有效产物，预览与执行均只生成后续单页改编并保留人工内容")
            for deleted_stage in ("page_slots", "preprocessing"):
                delete_input = {"project_id": deletion_project, "chapter_id": chapter_id, "stage": deleted_stage, "page_number": None}
                deletion = call("preview_delete_manga_pipeline_artifact", delete_input)
                call("delete_manga_pipeline_artifact", {**delete_input, "expected_impact_fingerprint": deletion["fingerprint"]}, destructive=True)
                cleared = call("list_manga_page_prompts", {"project_id": deletion_project, "chapter_id": chapter_id})
                assert len(cleared) == 2 and all(page["pipeline_stage"] == "content" and page["page_content"] is None and page["status"] == "pending" for page in cleared), cleared
                assert workspace(deletion_project)["chapters"]
                if deleted_stage == "page_slots":
                    manga_run(deletion_project, run_stage="page_slots")
                    assert len(call("list_manga_page_prompts", {"project_id": deletion_project, "chapter_id": chapter_id})) == 2
            passed("删除章节分页及预处理后页面投影仍可读取，工作区可打开，分页可以重新生成")

            unaffected_slots = call("list_manga_pipeline_artifacts", {"project_id": parallel_project, "chapter_id": chapter_id})
            retained_documents = workspace(deletion_project)["documents"]
            call("delete_manga_by_chapter", {"project_id": deletion_project, "chapter_id": chapter_id}, destructive=True)
            assert call("list_manga_page_prompts", {"project_id": deletion_project, "chapter_id": chapter_id}) == []
            assert call("list_manga_pipeline_artifacts", {"project_id": deletion_project, "chapter_id": chapter_id}) == []
            assert [item["record"] for item in workspace(deletion_project)["documents"]] == [item["record"] for item in retained_documents]
            assert call("list_manga_pipeline_artifacts", {"project_id": parallel_project, "chapter_id": chapter_id}) == unaffected_slots
            call("delete_manga_by_chapter", {"project_id": deletion_project, "chapter_id": chapter_id}, destructive=True)
            manga_run(deletion_project, run_stage="preprocessing")
            assert [item["stage"] for item in call("list_manga_pipeline_artifacts", {"project_id": deletion_project, "chapter_id": chapter_id})] == ["preprocessing"]
            passed("本章全部删除可重复执行，清空页面和阶段产物，保留改编文档及其他项目，并可重新预处理")

            failed_deletion_project = planning_project("失败分页全部删除后重建")
            preprocessing_model("a4n-page-planning-fail-content")
            manga_run(failed_deletion_project, run_stage="page_slots", expected_error="第 2 页剧情内容")
            assert call("get_manga_pipeline_status", {"project_id": failed_deletion_project, "chapter_id": chapter_id})["can_resume"]
            call("delete_manga_by_chapter", {"project_id": failed_deletion_project, "chapter_id": chapter_id}, destructive=True)
            reset_status = call("get_manga_pipeline_status", {"project_id": failed_deletion_project, "chapter_id": chapter_id})
            assert not reset_status["can_resume"] and not reset_status["has_checkpoint"], reset_status
            preprocessing_model("a4n-virtual-text")
            manga_run(failed_deletion_project, run_stage="preprocessing")
            passed("失败任务全部删除后清理旧续跑检查点，状态不再误导为可续跑，能够重新开始预处理")

            preview = call("preview_resume_manga_pipeline_request", {"project_id": recover_project, "chapter_number": 1, "run_stage": "preprocessing"})
            assert [step["state"] for step in preview["steps"]] == ["reused", "ready", "reused", "waiting_dependencies"]
            before = len(llm_calls())
            manga_run(recover_project, run_stage="preprocessing", resume=True)
            assert [name for name, _ in new_preprocessing_calls(before)] == [preprocessing_names[1], preprocessing_names[3]]
            artifacts = call("list_manga_pipeline_artifacts", {"project_id": recover_project, "chapter_id": chapter_id})
            assert len([item for item in artifacts if item["stage"] == "preprocessing"]) == 1
            passed("对白步骤仅纠正一次；失败不发布整包，保留事实与视觉检查点，续跑只补对白和综合两次调用")

            corrected_project = derive("comic", "漫画预处理单步纠正")
            preprocessing_model("a4n-preprocessing-recover-speech")
            before = len(llm_calls())
            manga_run(corrected_project, run_stage="preprocessing")
            corrected_calls = new_preprocessing_calls(before)
            assert len(corrected_calls) == 5 and [name for name, _ in corrected_calls].count(preprocessing_names[1]) == 2
            passed("单步字段或引用错误经过唯一纠正后可完成，其他步骤不重调")

            def weather_calls(offset):
                result = []
                for row in provider_log_entries(args.provider_log)[offset:]:
                    body = row.get("body", {})
                    messages = body.get("messages", body.get("input", []))
                    if not isinstance(messages, list):
                        continue
                    name = current_task_prompt_name(fixture_prompt_parts(messages)[1])
                    if name in preprocessing_names:
                        result.append((name, row))
                return result

            for provider, endpoint in (("openai", "/v1/chat/completions"), ("openai-responses", "/v1/responses")):
                for recovers in (False, True):
                    project = derive("comic", f"视觉重复字段 {provider} {recovers}")
                    model = "a4n-preprocessing-" + ("recover-weather" if recovers else "fail-weather")
                    preprocessing_model(model, provider=provider)
                    before = len(provider_log_entries(args.provider_log))
                    manga_run(project, run_stage="preprocessing", expected_error=None if recovers else "重复 JSON 字段 `weather`")
                    attempts = weather_calls(before)
                    assert len(attempts) == (5 if recovers else 4), attempts
                    visual = [row for name, row in attempts if name == preprocessing_names[2]]
                    assert len(visual) == 2 and all(row["path"] == endpoint for row in visual), visual
                    body = visual[1]["body"]
                    feedback = json.loads(message_text(body.get("messages", body.get("input"))[-1]["content"]))
                    assert feedback["json_rules"]["unique_object_keys"] is True
                    assert "/visual_anchors/scenes/0/weather" in feedback["validation_error"]
                    assert '"weather":null,"weather":null' in feedback["json_error_context"]["excerpt"]
                    assert feedback["max_correction_attempts"] == 1
                    artifacts = call("list_manga_pipeline_artifacts", {"project_id": project, "chapter_id": chapter_id})
                    if recovers:
                        assert len([item for item in artifacts if item["stage"] == "preprocessing"]) == 1, artifacts
                        assert [name for name, _ in attempts].count(preprocessing_names[3]) == 1
                        passed(f"{provider}：原始重复 weather 在唯一一次纠正中收到路径、原文与唯一字段规则后修正，四步共五次调用才发布产物")
                        continue
                    assert not any(item["stage"] == "preprocessing" for item in artifacts), artifacts
                    assert not any(name == preprocessing_names[3] for name, _ in attempts)
                    assert call("list_manga_page_prompts", {"project_id": project, "chapter_id": chapter_id}) == []
                    preprocessing_model("a4n-virtual-text", provider=provider)
                    preview = call("preview_resume_manga_pipeline_request", {"project_id": project, "chapter_number": 1, "run_stage": "preprocessing"})
                    assert [step["state"] for step in preview["steps"]] == ["reused", "reused", "ready", "waiting_dependencies"], preview
                    before = len(provider_log_entries(args.provider_log))
                    manga_run(project, run_stage="preprocessing", resume=True)
                    assert [name for name, _ in weather_calls(before)] == [preprocessing_names[2], preprocessing_names[3]]
                    artifacts = call("list_manga_pipeline_artifacts", {"project_id": project, "chapter_id": chapter_id})
                    assert len([item for item in artifacts if item["stage"] == "preprocessing"]) == 1, artifacts
                    passed(f"{provider}：持续重复 weather 仅纠正一次，不发布脏产物或启动综合；续跑保留事实/对白，只补视觉与综合")

            cancelled_project = derive("comic", "漫画预处理取消")
            preprocessing_model("a4n-preprocessing-slow")
            before = len(llm_calls())
            handle = call("start_manga_pipeline", {"project_id": cancelled_project, "chapter_number": 1, "run_stage": "preprocessing", "meta": {"start_request_id": str(uuid.uuid4())}})
            deadline = time.monotonic() + 20
            while len(new_preprocessing_calls(before)) < 3:
                assert time.monotonic() < deadline, "parallel preprocessing branches did not start"
                time.sleep(0.05)
            checked(cli, ["task", "cancel", handle["task_id"], "--attempt", str(handle["attempt"]), "--yes"], "cancel isolated preprocessing")
            deadline = time.monotonic() + 20
            while True:
                cancelled = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "cancelled preprocessing")[0]["data"]
                status = call("get_manga_pipeline_status", {"project_id": cancelled_project, "chapter_id": chapter_id})
                if cancelled["handle"]["status"] == "cancelled" and status["pipeline_status"] == "cancelled":
                    break
                assert time.monotonic() < deadline, (cancelled, status)
                time.sleep(0.05)
            assert cancelled["handle"]["status"] == "cancelled", cancelled
            assert not any(item["stage"] == "preprocessing" for item in call("list_manga_pipeline_artifacts", {"project_id": cancelled_project, "chapter_id": chapter_id}))
            assert not any(name == preprocessing_names[3] for name, _ in new_preprocessing_calls(before))
            passed("取消同时停止在途提取分支，不启动综合，也不发布不完整预处理产物")
            preprocessing_model("a4n-virtual-text")
            comic_adaptation = adopt(generate(comic))
            game_adaptation = adopt(generate(game))
            assert comic_adaptation["document"]["target"] == "manga" and game_adaptation["document"]["target"] == "galgame"
            assert comic_adaptation["inputs"]["video_reference"] is None
            passed("漫画和 Galgame 在没有视频项目或拍摄剧本时独立生成并采用改编计划")

            recovery_project = derive("galgame", "改编尾部 JSON 纠正")
            snapshot = copy.deepcopy(mock_runtime)
            snapshot["llm"].update(provider="openai-responses", model="a4n-output-recover-trailing-json", max_output_tokens=20000)
            secure_json(work / "runtime.json", snapshot)
            checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "configure Responses correction")
            offset = len(provider_log_entries(args.provider_log))
            recovered = generate(recovery_project)
            attempts = [row for row in provider_log_entries(args.provider_log)[offset:]
                        if row.get("body", {}).get("model") == "a4n-output-recover-trailing-json"]
            assert len(attempts) == 2 and all(row["path"] == "/v1/responses" for row in attempts), attempts
            assert all(row["body"]["text"]["format"]["name"] == "chapter_media_adaptation_plan" for row in attempts)
            feedback = json.loads(message_text(attempts[1]["body"]["input"][-1]["content"]))
            assert feedback["kind"] == "model_output_correction" and feedback["previous_response_chars"] > 24_676
            assert "trailing characters" in feedback["validation_error"]
            assert len(feedback["previous_response_excerpt"]) <= 8_000
            assert feedback["previous_response_excerpt"].endswith('{"unexpected":"TRAILING_ADAPTATION_MARKER"}')
            assert latest(recovery_project, "adaptation_plan", "draft")["document"] == recovered["document"]
            assert "TRAILING_ADAPTATION_MARKER" not in json.dumps(recovered["document"])
            adopt(recovered)
            passed("Responses 改编的长 JSON 尾部非法内容被拒绝，纠正反馈保留错误尾部，仅一次纠正后持久化并采用有效计划")
            preprocessing_model("a4n-virtual-text")

            before = len(llm_calls())
            provider_request_offset = len(provider_log_entries(args.provider_log))
            manga_run()
            pages = call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id})
            assert len(pages) == 2 and len(llm_calls()) - before == len(pages) * 5 + 1
            assert all(page["page_content"] and page["page_adaptation"] and page["page_plan"] and page["reference_discovery"] and page["drawing_prompt"] for page in pages)
            assert pages[0]["source"]["adaptation_plan_id"] == comic_adaptation["id"]
            assert all(page["generated_image_id"] for page in pages)
            image_calls = [row for row in provider_log_entries(args.provider_log)[provider_request_offset:] if "/images/" in row.get("path", "")]
            assert len(image_calls) == len(pages), image_calls
            manga_artifacts = workspace(comic)["artifacts"]
            assert not any(artifact["stale"] for artifact in manga_artifacts), manga_artifacts
            passed("漫画一次完成页面槽位、页面改编、分镜、绘图提示词和成图")

            deletion_cases = [
                ("preprocessing", None, {"preprocessing", "page_slots", "page_content", "page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "content", 11),
                ("page_slots", None, {"page_slots", "page_content", "page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "content", 11),
                ("page_content", 1, {"page_content", "page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "content", 4),
                ("page_adaptation", 1, {"page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "adaptation", 4),
                ("storyboard", 1, {"storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "storyboard", 3),
                ("reference_bindings", 1, {"reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "reference_discovery", 2),
                ("reference_discovery", 1, {"reference_discovery", "drawing_prompt", "image"}, "reference_discovery", 2),
                ("drawing_prompt", 1, {"drawing_prompt", "image"}, "drawing_prompt", 1),
                ("image", 1, {"image"}, "image", 0),
                ("page_content", None, {"page_content", "page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "content", 8),
                ("page_adaptation", None, {"page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "adaptation", 8),
                ("storyboard", None, {"storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "storyboard", 6),
                ("reference_bindings", None, {"reference_bindings", "reference_discovery", "drawing_prompt", "image"}, "reference_discovery", 4),
                ("reference_discovery", None, {"reference_discovery", "drawing_prompt", "image"}, "reference_discovery", 4),
                ("drawing_prompt", None, {"drawing_prompt", "image"}, "drawing_prompt", 2),
                ("image", None, {"image"}, "image", 0),
            ]
            projection_fields = {"page_content": "page_content", "page_adaptation": "page_adaptation", "storyboard": "page_plan",
                "reference_bindings": "reference_bindings", "reference_discovery": "reference_discovery", "drawing_prompt": "drawing_prompt", "image": "generated_image_id"}
            for deleted_stage, target_page, removed_stages, reset_stage, expected_calls in deletion_cases:
                audited_project = planning_project(f"删除阶段审查 {deleted_stage} {target_page}")
                manga_run(audited_project)
                artifact_scope = {"project_id": audited_project, "chapter_id": chapter_id}
                original_artifacts = call("list_manga_pipeline_artifacts", artifact_scope)
                original_pages = call("list_manga_page_prompts", artifact_scope)
                assert len(original_pages) == 2 and all(page["generated_image_id"] for page in original_pages)
                delete_scope = {**artifact_scope, "stage": deleted_stage, "page_number": target_page}
                impact = call("preview_delete_manga_pipeline_artifact", delete_scope)
                expected_deleted = [item for item in original_artifacts if item["stage"] in removed_stages and (target_page is None or item["page_number"] == target_page)]
                assert {item["artifact_id"] for item in [impact["target"], *impact["downstream"]] if item["artifact_id"] is not None} == {item["id"] for item in expected_deleted}, (deleted_stage, impact)
                call("delete_manga_pipeline_artifact", {**delete_scope, "expected_impact_fingerprint": impact["fingerprint"]}, destructive=True)
                retained = call("list_manga_pipeline_artifacts", artifact_scope)
                assert retained == [item for item in original_artifacts if item not in expected_deleted], deleted_stage
                cleared_pages = call("list_manga_page_prompts", artifact_scope)
                assert [page["id"] for page in cleared_pages] == [page["id"] for page in original_pages], deleted_stage
                for old, cleared in zip(original_pages, cleared_pages, strict=True):
                    if target_page is not None and old["page_number"] != target_page:
                        assert cleared == old, deleted_stage
                        continue
                    assert cleared["pipeline_stage"] == reset_stage and cleared["pipeline_status"] == "pending" and cleared["status"] == "pending", (deleted_stage, cleared)
                    assert cleared["pipeline_error"] is None
                    assert cleared["dialogue_text"] == (None if "storyboard" in removed_stages else old["dialogue_text"]), deleted_stage
                    for stage_id, field in projection_fields.items():
                        if stage_id in removed_stages:
                            assert not cleared[field], (deleted_stage, field, cleared[field])
                        else:
                            assert cleared[field] == old[field], (deleted_stage, field)
                assert workspace(audited_project)["chapters"]
                if deleted_stage == "preprocessing":
                    assert next(view for view in workspace(audited_project)["documents"] if view["record"]["status"] == "adopted")["stale"]
                    manga_run(audited_project, run_stage="preprocessing")
                request_offset = len(llm_calls())
                manga_run(audited_project, stop_after="drawing_prompt", page_number=target_page)
                assert len(llm_calls()) - request_offset == expected_calls, (deleted_stage, len(llm_calls()) - request_offset)
                regenerated = call("list_manga_page_prompts", artifact_scope)
                assert all(page["page_content"] and page["page_adaptation"] and page["page_plan"] and page["reference_discovery"] and page["drawing_prompt"] for page in regenerated), deleted_stage
                manga_run(audited_project, run_stage="image", page_number=1)
                assert next(page for page in call("list_manga_page_prompts", artifact_scope) if page["page_number"] == 1)["generated_image_id"]
                passed(f"逐阶段删除 {deleted_stage}（{'本章全部页面' if target_page is None else f'第 {target_page} 页'}）：级联范围、页面身份、保留上游及其他页、布局状态和真实重新生成全部通过")

            mixed_project = planning_project("本章阶段删除的并发修改与未开始页")
            manga_run(mixed_project, stop_after="page_adaptation")
            manga_run(mixed_project, stop_after="drawing_prompt", page_number=1)
            mixed_scope = {"project_id": mixed_project, "chapter_id": chapter_id}
            mixed_pages = call("list_manga_page_prompts", mixed_scope)
            assert mixed_pages[0]["drawing_prompt"] and not mixed_pages[1]["page_plan"]
            bulk_scope = {**mixed_scope, "stage": "drawing_prompt", "page_number": None}
            impact = call("preview_delete_manga_pipeline_artifact", bulk_scope)
            edited_prompt = copy.deepcopy(mixed_pages[0]["drawing_prompt"])
            edited_prompt["image_prompt"] += "，确认期间修改后的提示词"
            call("save_manga_pipeline_artifact", {**mixed_scope, "stage": "drawing_prompt", "page_number": 1,
                "expected_prompt_updated_at": mixed_pages[0]["updated_at"], "payload": edited_prompt})
            before_rejection = call("list_manga_pipeline_artifacts", mixed_scope)
            pages_before_rejection = call("list_manga_page_prompts", mixed_scope)
            call("delete_manga_pipeline_artifact", {**bulk_scope, "expected_impact_fingerprint": impact["fingerprint"]}, destructive=True, reject="范围已变化")
            assert call("list_manga_pipeline_artifacts", mixed_scope) == before_rejection
            assert call("list_manga_page_prompts", mixed_scope) == pages_before_rejection
            impact = call("preview_delete_manga_pipeline_artifact", bulk_scope)
            call("delete_manga_pipeline_artifact", {**bulk_scope, "expected_impact_fingerprint": impact["fingerprint"]}, destructive=True)
            after_mixed_delete = call("list_manga_page_prompts", mixed_scope)
            assert after_mixed_delete[0]["drawing_prompt"] is None
            assert {key: value for key, value in after_mixed_delete[1].items() if key != "updated_at"} == {key: value for key, value in mixed_pages[1].items() if key != "updated_at"}
            assert call("list_manga_pipeline_artifacts", mixed_scope) == [item for item in before_rejection if item["stage"] != "drawing_prompt"]
            passed("本章阶段批量删除在确认期间发生修改时原子拒绝；重新预览后只清理目标，不推进尚未开始该阶段的页面")

            deleted_plan = latest(audited_project, "adaptation_plan", "adopted")
            document_scope = {"project_id": audited_project, "record_id": deleted_plan["id"], "kind": "adaptation_plan", "expected_revision": deleted_plan["revision"]}
            plan_impact = call("preview_delete_media_authoring_document", document_scope)
            assert len(plan_impact["manga_pages"]) == 2
            assert [item["stage"] for item in plan_impact["manga_chapter_artifacts"]] == ["page_slots"], plan_impact
            call("delete_media_authoring_document", {**document_scope, "expected_impact_fingerprint": plan_impact["fingerprint"]}, destructive=True)
            assert call("list_manga_page_prompts", artifact_scope) == []
            assert [item["stage"] for item in call("list_manga_pipeline_artifacts", artifact_scope)] == ["preprocessing"]
            adopt(generate(audited_project))
            manga_run(audited_project, stop_after="drawing_prompt")
            assert len(call("list_manga_page_prompts", artifact_scope)) == 2
            passed("删除已采用改编计划清理章节分页和全部页级产物，保留预处理，重新采用计划后可以完整生成")

            for _ in range(2):
                request_offset = len(provider_log_entries(args.provider_log))
                handle = call("start_image_generation", {"project_id": audited_project,
                    "meta": {"start_request_id": str(uuid.uuid4())},
                    "operation": {"kind": "page", "chapter_number": 1, "page_number": 1, "width": 1024, "height": 1024}})
                checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "standalone page image")
                generated = call("list_manga_page_prompts", artifact_scope)
                assert generated[0]["generated_image_id"]
                assert any("/images/" in row.get("path", "") for row in provider_log_entries(args.provider_log)[request_offset:])
                assert not any(item["stage"] == "image" for item in call("list_manga_pipeline_artifacts", artifact_scope))
                image_scope = {**artifact_scope, "stage": "image", "page_number": 1}
                image_impact = call("preview_delete_manga_pipeline_artifact", image_scope)
                assert image_impact["target"]["artifact_id"] is None and image_impact["downstream"] == []
                call("delete_manga_pipeline_artifact", {**image_scope, "expected_impact_fingerprint": image_impact["fingerprint"]}, destructive=True)
                cleared = call("list_manga_page_prompts", artifact_scope)
                assert cleared[0]["generated_image_id"] is None and cleared[0]["drawing_prompt"]
                assert cleared[1] == generated[1]
            passed("独立单页生图没有流水线 artifact 也可预览和删除；连续两次生成与删除都真实调用图片模型，不恢复已删图片")

            placement_project = planning_project("分镜位置合同一致性")
            manga_run(placement_project, stop_after="page_adaptation")
            preprocessing_model("a4n-output-recover-manga-placement")
            before = len(llm_calls())
            manga_run(placement_project, stop_after="storyboard", run_stage="storyboard", page_number=1)
            attempts = llm_calls()[before:]
            assert len(attempts) == 2, attempts
            assert all(current_task_prompt_name(fixture_prompt_parts(row["body"]["messages"])[1]) == "manga_page_storyboard" for row in attempts)
            feedback = json.loads(attempts[1]["body"]["messages"][-1]["content"])
            assert feedback["kind"] == "model_output_correction"
            assert "placement" in feedback["validation_error"] and "upper_right" in feedback["previous_response_excerpt"]
            for row in attempts:
                placements = row["body"]["response_format"]["json_schema"]["schema"]["$defs"]["MangaTextPlacementV2"]["enum"]
                assert len(placements) == 8 and "upper_right" not in placements, placements
            saved_page = next(page for page in call("list_manga_page_prompts", {"project_id": placement_project, "chapter_id": chapter_id}) if page["page_number"] == 1)
            assert "upper_right" not in json.dumps(saved_page["page_plan"])
            passed("分镜请求 Schema 与提示词均拒绝位置别名，原始输出触发唯一纠正后才保存，不隐式改写")
            preprocessing_model("a4n-virtual-text")

            storyboard_preview = call("preview_start_manga_pipeline_request", {
                "project_id": comic,
                "chapter_number": 1,
                "page_number": 1,
                "stop_after": "storyboard",
                "run_stage": "storyboard",
            })
            assert len(storyboard_preview["steps"]) == 1, storyboard_preview
            assert storyboard_preview["steps"][0]["prompt_name"] == "manga_page_storyboard"
            passed("单阶段请求预览只返回所选分镜步骤")

            before_image_rerun = len(llm_calls())
            before_image_requests = len(provider_log_entries(args.provider_log))
            manga_run(comic, stop_after="image", page_number=1, run_stage="image", expected_error="请先删除")
            assert len(provider_log_entries(args.provider_log)) == before_image_requests
            preserved_pages = call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id})
            assert next(page for page in preserved_pages if page["page_number"] == 1)["generated_image_id"] == next(page for page in pages if page["page_number"] == 1)["generated_image_id"]
            delete_input = {"project_id": comic, "chapter_id": chapter_id, "stage": "image", "page_number": 1}
            deletion = call("preview_delete_manga_pipeline_artifact", delete_input)
            assert deletion["downstream"] == [], deletion
            call("delete_manga_pipeline_artifact", {
                **delete_input, "expected_impact_fingerprint": deletion["fingerprint"],
            }, destructive=True)
            previous_image_id = next(page for page in pages if page["page_number"] == 1)["generated_image_id"]
            manga_run(comic, stop_after="image", page_number=1, run_stage="image")
            assert len(llm_calls()) == before_image_rerun
            regenerated_pages = call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id})
            new_image_id = next(page for page in regenerated_pages if page["page_number"] == 1)["generated_image_id"]
            assert new_image_id and new_image_id != previous_image_id
            assert len(provider_log_entries(args.provider_log)) > before_image_requests
            pages = regenerated_pages
            passed("已有成图在请求模型前拒绝覆盖；显式删除后重新生图并复用文本阶段")

            scoped_comic = derive("comic", "单页范围验收漫画")
            manga_run(scoped_comic, run_stage="preprocessing")
            scoped_adaptation = adopt(generate(scoped_comic))
            assert scoped_adaptation["document"]["target"] == "manga", scoped_adaptation
            manga_run(scoped_comic, stop_after="page_slots")
            scoped_pages = call("list_manga_page_prompts", {"project_id": scoped_comic, "chapter_id": chapter_id})
            assert len(scoped_pages) == 2
            assert all(page["page_content"] for page in scoped_pages)
            before_selected_page = len(llm_calls())
            manga_run(scoped_comic, stop_after="storyboard", page_number=2)
            assert len(llm_calls()) - before_selected_page == 2
            scoped_pages = call("list_manga_page_prompts", {"project_id": scoped_comic, "chapter_id": chapter_id})
            first_page = next(page for page in scoped_pages if page["page_number"] == 1)
            selected_page = next(page for page in scoped_pages if page["page_number"] == 2)
            assert first_page.get("page_adaptation") is None and first_page.get("page_plan") is None
            assert selected_page.get("page_adaptation") and selected_page.get("page_plan")
            before_next_page = len(llm_calls())
            manga_run(scoped_comic, stop_after="storyboard", page_number=1)
            assert len(llm_calls()) - before_next_page == 2
            scoped_pages = call("list_manga_page_prompts", {"project_id": scoped_comic, "chapter_id": chapter_id})
            first_page = next(page for page in scoped_pages if page["page_number"] == 1)
            selected_page = next(page for page in scoped_pages if page["page_number"] == 2)
            assert first_page.get("page_adaptation") and first_page.get("page_plan")
            assert selected_page.get("page_adaptation") and selected_page.get("page_plan")
            passed("单页分镜按选中页执行，连续处理另一页时复用已完成的整章阶段")

            delete_input = {"project_id": scoped_comic, "chapter_id": chapter_id, "stage": "storyboard", "page_number": 2}
            deletion = call("preview_delete_manga_pipeline_artifact", delete_input)
            call("delete_manga_pipeline_artifact", {
                **delete_input,
                "expected_impact_fingerprint": deletion["fingerprint"],
            }, destructive=True)
            before_visual_regeneration = len(llm_calls())
            manga_run(scoped_comic, stop_after="reference_discovery", page_number=2, run_stage="storyboard")
            assert len(llm_calls()) - before_visual_regeneration == 2
            passed("删除本页分镜后组合阶段重新调用分镜与参考发现，不回放已完成工作流的旧答案")

            for _ in range(2):
                deletion = call("preview_delete_manga_pipeline_artifact", delete_input)
                call("delete_manga_pipeline_artifact", {
                    **delete_input,
                    "expected_impact_fingerprint": deletion["fingerprint"],
                }, destructive=True)
                before_single_regeneration = len(llm_calls())
                manga_run(scoped_comic, stop_after="storyboard", page_number=2, run_stage="storyboard")
                assert len(llm_calls()) - before_single_regeneration == 1
            passed("相同页相同单阶段两次删除后生成均产生新模型调用，不隐式续接已完成检查点")

            game_input = {"project_id": game, "task_id": str(uuid.uuid4()), "resume_from_checkpoint": True, "start_from_stage": None, "stop_after": "branch_design"}
            before_galgame = len(provider_log_entries(args.provider_log))
            assert call("start_galgame_pipeline", game_input) is None
            assert call("get_galgame_script", {"project_id": game}) is None
            branch = latest(game, "galgame_branch_plan", "draft")
            edited_branch = copy.deepcopy(branch["document"])
            edited_branch["choices"][0]["question"] = "作者保留的选择：相信物证还是口供？"
            branch = adopt(save(branch, edited_branch))
            script = call("start_galgame_pipeline", {**game_input, "task_id": str(uuid.uuid4()), "stop_after": "script"})
            assert script and script["scenes"]
            assert all(line["presentation_json"]["schema_version"] == "galgame_line_presentation_v3" for scene in script["scenes"] for line in scene["lines"])
            assert not workspace(game)["artifacts"][0]["stale"]
            assert not any("/images/" in row["path"] or "/videos" in row["path"] for row in provider_log_entries(args.provider_log)[before_galgame:])
            passed("Galgame 分支阶段单独停止并保留人工选择；剧本使用本地单元；一键文本流程不生成付费媒体")

            copies = {}
            for project_id in (comic, game):
                copied = call("duplicate_project", {"project_id": project_id, "new_title": "保留全部计划的副本"})
                copies[project_id] = copied["id"]
                original_docs, copied_docs = workspace(project_id)["documents"], workspace(copied["id"])["documents"]
                assert len(copied_docs) == len(original_docs)
                assert set(view["record"]["id"] for view in original_docs).isdisjoint(view["record"]["id"] for view in copied_docs)
                copied_workspace = workspace(copied["id"])
                stale_artifacts = [artifact for artifact in copied_workspace["artifacts"] if artifact["stale"]]
                assert not stale_artifacts, {
                    "artifacts": stale_artifacts,
                    "documents": [
                        {
                            "id": view["record"]["id"],
                            "kind": view["record"]["kind"],
                            "status": view["record"]["status"],
                            "revision": view["record"]["revision"],
                            "stale": view["stale"],
                        }
                        for view in copied_docs
                    ],
                }
            copied_pages = call("list_manga_page_prompts", {"project_id": copies[comic], "chapter_id": chapter_id})
            assert copied_pages[0]["image_prompt"] == pages[0]["image_prompt"] and copied_pages[0]["id"] != pages[0]["id"]
            copied_script = call("get_galgame_script", {"project_id": copies[game]})
            assert copied_script["scenes"][0]["scene"]["id"] != script["scenes"][0]["scene"]["id"]
            passed("复制漫画和游戏保留草稿、采用版本、脚本和人工修改，所有可编辑身份独立映射")

            film, second_film = derive("video"), derive("video")
            resource = {"project_id": film, "source_chapter_id": chapter_id}
            call("get_chapter_visual_plan", {"project_id": root_id, "source_chapter_id": chapter_id}, reject="视频")
            view = call("get_chapter_visual_plan", resource)
            plan = view["starter_plan"]
            plan["segments"][0]["scene_binding"] = {"kind": "scene", "scene_profile_id": view["scene_choices"][0]["id"]}
            plan["segments"][0]["prompt"] = "林岚在旧车站通道观察怀表，周衡在后方收紧工具箱提手。"

            def shooting(plan):
                draft = call("save_chapter_visual_plan_draft", {**resource, "authoring_mode": "manual", "record_id": None, "expected_revision": None, "plan": plan})
                confirmed = call("confirm_chapter_visual_plan", {**resource, "record_id": draft["record"]["id"], "expected_revision": draft["record"]["revision"]}, destructive=True)["visual_plan"]
                handle = call("start_chapter_shooting_script_generation", {**resource, "visual_plan_id": confirmed["record"]["id"], "shot_ids": [], "mode": "full", "meta": {"start_request_id": str(uuid.uuid4())}})
                checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "shooting script")
                return call("get_chapter_shooting_script", resource)

            first_script = shooting(plan)
            assert call("get_chapter_shooting_script", {"project_id": second_film, "source_chapter_id": chapter_id}) is None
            references = call("list_video_adaptation_references", {"project_id": comic, "chapter_id": chapter_id})
            selection = next(row["selection"] for row in references if row["selection"]["video_project_id"] == film)
            borrowed = adopt(generate(comic, comic_adaptation, selection))
            frozen_hash = borrowed["inputs"]["video_reference"]["script_sha256"]
            assert workspace(comic)["artifacts"][0]["stale"]
            assert call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id}) == pages
            plan["points"][0]["label"] = "作者修改后的导演方案"
            newer_script = shooting(plan)
            assert newer_script["record"]["revision"] > first_script["record"]["revision"]
            ref_view = next(view for view in workspace(comic)["documents"] if view["record"]["id"] == borrowed["id"])
            assert ref_view["video_reference_status"] == "updated" and ref_view["record"]["inputs"]["video_reference"]["script_sha256"] == frozen_hash
            passed("视频项目导演和剧本彼此隔离；冻结参考不随制作者更新；采用新改编只提示下游过期并保留原页面")

            exported = call("export_project_family_data", {"root_project_id": root_id, "include_derived_projects": True})
            imported = call("import_project_family_upload", {"file_name": exported["file_name"], "data_base64": exported["data_base64"]})
            imported_comic = next(project for project in imported if project["title"] == "独立comic")
            imported_game = next(project for project in imported if project["title"] == "独立galgame")
            report["ui_project_ids"] = {
                "comic": imported_comic["id"], "galgame": imported_game["id"],
                "video": next(project["id"] for project in imported if project["project_kind"] == "video"),
            }
            imported_workspace = workspace(imported_comic["id"])
            imported_reference = next(view for view in imported_workspace["documents"] if view["record"]["status"] == "adopted" and view["record"]["kind"] == "adaptation_plan")
            assert imported_reference["record"]["inputs"]["video_reference"]["script_sha256"] == frozen_hash
            assert imported_reference["video_reference_status"] == "updated"
            assert imported_workspace["chapters"][0]["source"]["source_chapter_id"] != chapter_id
            imported_game_script = call("get_galgame_script", {"project_id": imported_game["id"]})
            for scene in imported_game_script["scenes"]:
                for line in scene["lines"]:
                    reference = line["presentation_json"]["beat_ref"]
                    if reference:
                        assert reference["source_chapter_id"] != chapter_id
            passed("真实项目包往返映射章节、计划、分支、演出引用，过期视频参考仍保留原始哈希")

            def bundle_manifest(exported):
                with zipfile.ZipFile(io.BytesIO(base64.b64decode(exported["data_base64"]))) as archive:
                    return json.loads(archive.read("a4n-project-bundle.json"))

            bundle = call("export_project_bundle_data", {"project_id": root_id, "include_derived_projects": True})
            before_bundle = bundle_manifest(bundle)
            bundle_import = call("import_project_bundle_upload", {"file_name": bundle["file_name"], "data_base64": bundle["data_base64"]})
            assert bundle_import["workflow_runs_imported"] == len(before_bundle["history"]["workflow_runs"]) > 0
            bundle_root = next(project for project in bundle_import["project_summaries"] if project["project_kind"] == "text")
            after_bundle = bundle_manifest(call("export_project_bundle_data", {"project_id": bundle_root["id"], "include_derived_projects": True}))
            before_checkpoints = [row["checkpoint_json"] for row in before_bundle["history"]["workflow_checkpoints"]]
            after_checkpoints = [row["checkpoint_json"] for row in after_bundle["history"]["workflow_checkpoints"]]
            assert len(before_checkpoints) == len(after_checkpoints)
            # Structured references move to imported entities; provider evidence stays verbatim.
            def call_evidence(checkpoints):
                return sorted(json.dumps({key: value[key] for key in ("response_content", "usage", "finish_reason")}, sort_keys=True)
                    for value in checkpoints if isinstance(value, dict) and "response_content" in value)
            assert call_evidence(before_checkpoints) and call_evidence(before_checkpoints) == call_evidence(after_checkpoints)
            before_calls = before_bundle["history"]["generation_runs"]
            after_calls = after_bundle["history"]["generation_runs"]
            assert len(before_calls) == len(after_calls)
            assert sorted(json.dumps(row["raw_response_json"], sort_keys=True) for row in before_calls) == sorted(json.dumps(row["raw_response_json"], sort_keys=True) for row in after_calls)
            assert len(after_bundle["family"]["authoring_documents"]) == len(before_bundle["family"]["authoring_documents"])
            assert len(after_bundle["history"]["generation_run_targets"]) == len(before_bundle["history"]["generation_run_targets"])
            passed("现代完整项目包往返保留模型断点原文、调用历史和全部创作文档")

            scoped = call("export_derived_project_data", {"derived_project_id": film})
            with zipfile.ZipFile(io.BytesIO(base64.b64decode(scoped["data_base64"]))) as archive:
                files = {name: archive.read(name) for name in archive.namelist()}
            legacy = json.loads(files["a4n-project-family.json"])
            legacy["version"] = "project_family_v7"
            del legacy["authoring_documents"], legacy["authoring_receipts"]
            for table in ("chapter_visual_plans", "chapter_shooting_scripts"):
                for row in legacy[table]:
                    del row["project_id"]
            files["a4n-project-family.json"] = json.dumps(legacy, ensure_ascii=False).encode()
            archive_bytes = io.BytesIO()
            with zipfile.ZipFile(archive_bytes, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                for name, value in files.items():
                    archive.writestr(name, value)
            legacy_imported = call("import_project_family_upload", {"file_name": scoped["file_name"], "data_base64": base64.b64encode(archive_bytes.getvalue()).decode()})
            assert any(project["project_kind"] == "video" for project in legacy_imported)
            passed("缺少新必填字段的 v7 项目包在反序列化之前升级为项目独立归属")

            call("delete_project", {"projectId": film}, raw=True, destructive=True)
            retained = next(view for view in workspace(comic)["documents"] if view["record"]["id"] == borrowed["id"])
            assert retained["video_reference_status"] == "unavailable" and retained["record"]["inputs"]["video_reference"]["script_sha256"] == frozen_hash
            call("delete_project", {"projectId": copies[game]}, raw=True, destructive=True)
            assert call("get_galgame_script", {"project_id": game})["id"] == script["id"]
            call("detach_derived_project", {"derived_project_id": copies[comic]}, destructive=True)
            assert workspace(copies[comic])["documents"]
            call("reset_project_generated_data", {"projectId": root_id}, raw=True, destructive=True)
            assert workspace(comic)["documents"] and call("get_galgame_script", {"project_id": game})["id"] == script["id"]
            passed("删除制作者或其他副本、分离项目及重置原作均保留派生创作和固定原文")
            imported_root = next(project for project in imported if project["project_kind"] == "text")
            report["ui_root_id"] = imported_root["id"]
            report["ui_mutation_project_ids"] = {}
            for purpose in ("draft", "image", "reference"):
                current = call("create_derived_project", {
                    "root_project_id": imported_root["id"], "derived_kind": "comic",
                    "title": f"浏览器{purpose}操作验收", "description": "current browser mutation fixture",
                    "start_chapter": 1, "end_chapter": 1,
                })
                current_id = current["id"]
                report["projects"].append(current_id)
                manga_run(current_id, run_stage="preprocessing")
                adopt(generate(current_id))
                if purpose == "image":
                    manga_run(current_id)
                if purpose == "reference":
                    manga_run(current_id, stop_after="reference_discovery")
                    reference_chapter = workspace(current_id)["chapters"][0]["source"]["source_chapter_id"]

                    def reference_page():
                        return next(page for page in call("list_manga_page_prompts", {
                            "project_id": current_id, "chapter_id": reference_chapter,
                        }) if page["page_number"] == 1)

                    reference = reference_page()
                    discovery = copy.deepcopy(reference["reference_discovery"])
                    discovery["references"] = []
                    discovery["requirements"] = [{
                        "target_kind": "object", "target_name": "断裂树根",
                        "description": "断裂的树根位于近景，雨水与泥浆覆盖表面，遮挡后方藏身位置。",
                        "usage": "保持人物伏在树根后方的空间关系；不添加追兵或新剧情。",
                    }]
                    call("save_manga_pipeline_artifact", {"project_id": current_id, "chapter_id": reference_chapter,
                        "stage": "reference_discovery", "page_number": 1, "payload": discovery,
                        "expected_prompt_updated_at": reference["updated_at"]})

                    def reference_input(page, **overrides):
                        return {"project_id": current_id, "meta": {"start_request_id": str(uuid.uuid4())},
                            "operation": {"kind": "manga_reference", "options": {
                                "prompt_id": page["id"], "target_kind": "object", "target_name": "断裂树根",
                                "expected_updated_at": page["updated_at"], "width": 1024, "height": 1024,
                                **overrides,
                            }}}

                    reference = reference_page()
                    assert "system_reference_images" not in reference
                    duplicated = copy.deepcopy(discovery)
                    duplicated["requirements"].append(copy.deepcopy(duplicated["requirements"][0]))
                    call("save_manga_pipeline_artifact", {"project_id":current_id, "chapter_id":reference_chapter,
                        "stage":"reference_discovery", "page_number":1, "payload":duplicated,
                        "expected_prompt_updated_at":reference["updated_at"]}, reject="目标不唯一")
                    assert reference_page()["updated_at"] == reference["updated_at"]
                    passed("参考发现只有一个权威返回字段；重复目标保存被拒绝，页面及版本保持不变")
                    before = len(provider_log_entries(args.provider_log))
                    call("start_image_generation", reference_input(reference, target_name="未发现的道具"), reject="未找到该参考图需求")
                    forged = reference_input(reference)
                    forged["project_id"] = report["ui_mutation_project_ids"]["draft"]
                    call("start_image_generation", forged, reject="不属于当前项目")
                    call("start_image_generation", reference_input(reference, expected_updated_at="2020-01-01 00:00:00"), reject="已被其他操作修改")
                    assert len(provider_log_entries(args.provider_log)) == before
                    passed("发现需求生图在 Provider 请求前拒绝未知目标、跨项目页面及过期版本")

                    def reference_image_model(model):
                        snapshot = copy.deepcopy(mock_runtime)
                        snapshot["image"]["model"] = model
                        secure_json(work / "runtime.json", snapshot)
                        checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "reference image model")

                    def reference_terminal(handle):
                        deadline = time.monotonic() + 60
                        while True:
                            task = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "reference terminal")[0]["data"]
                            if task["handle"]["status"] in {"completed", "failed", "cancelled", "interrupted"}:
                                return task
                            assert time.monotonic() < deadline
                            time.sleep(0.05)

                    reference_image_model("a4n-reference-image-fail")
                    images_before = call("list_images_by_project", {"project_id": current_id})
                    handle = call("start_image_generation", reference_input(reference))
                    failed = reference_terminal(handle)
                    assert failed["handle"]["status"] == "failed" and "reference image fixture failure" in json.dumps(failed), failed
                    assert reference_page()["reference_discovery"] == discovery
                    assert call("list_images_by_project", {"project_id": current_id}) == images_before
                    passed("参考图 Provider 失败后任务明确失败，不保存图片、不产生绑定，页面仍可重试")

                    reference_image_model("a4n-reference-image-slow")
                    offset = len(provider_log_entries(args.provider_log))
                    handle = call("start_image_generation", reference_input(reference))
                    deadline = time.monotonic() + 15
                    while len(provider_log_entries(args.provider_log)) == offset:
                        assert time.monotonic() < deadline
                        time.sleep(0.02)
                    call("save_manga_pipeline_artifact", {"project_id": current_id, "chapter_id": reference_chapter,
                        "stage": "reference_discovery", "page_number": 1, "payload": discovery,
                        "expected_prompt_updated_at": reference["updated_at"]})
                    conflicted = reference_terminal(handle)
                    assert conflicted["handle"]["status"] == "failed" and "自动绑定失败" in json.dumps(conflicted, ensure_ascii=False), conflicted
                    assert reference_page()["reference_discovery"] == discovery
                    assert len(call("list_images_by_project", {"project_id": current_id})) == len(images_before) + 1
                    passed("参考图生成期间页面版本变化不会覆盖新页面，已生成素材保留，任务明确提示自动绑定失败")
                    reference_image_model("a4n-virtual-image")
                    reference = reference_page()
                    initial_reference = reference
                    for count in (1, 2):
                        offset = len(provider_log_entries(args.provider_log))
                        handle = call("start_image_generation", reference_input(reference))
                        checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "reference image task")
                        rows = provider_log_entries(args.provider_log)[offset:]
                        assert len(rows) == 1 and "/images/generations" in rows[0]["path"], rows
                        assert discovery["requirements"][0]["description"] in rows[0]["body"]["prompt"]
                        assert discovery["requirements"][0]["usage"] in rows[0]["body"]["prompt"]
                        reference = reference_page()
                        assert len(reference["reference_discovery"]["references"]) == count
                        assert reference["generated_image_id"] is None
                        bound = reference["reference_discovery"]["references"][-1]
                        assert bound["target_name"] == "断裂树根" and bound["asset"]["asset_kind"] == "generated_image"
                        images = call("list_images_by_project", {"project_id": current_id})
                        image = next(row for row in images if row["id"] == bound["asset"]["asset_id"])
                        assert image["image_type"] == "chapter_reference" and image["file_path"] == bound["asset"]["image_path"]
                        assert image["metadata_json"]["requirement"] == discovery["requirements"][0]
                    passed("发现需求分别生成两张参考图：各仅一次图片调用、不调用文本模型，持久化素材和正确绑定，保留漫画页成图状态")
                    before = len(provider_log_entries(args.provider_log))
                    call("start_image_generation", reference_input(initial_reference), reject="已被其他操作修改")
                    call("start_image_generation", reference_input(reference), reject="最多绑定两张")
                    assert len(provider_log_entries(args.provider_log)) == before
                    passed("已更新页面的旧请求和每个需求第三张参考图在请求 Provider 前被拒绝")
                    invalid_discovery = copy.deepcopy(reference["reference_discovery"])
                    used_ids = {item["asset"]["asset_id"] for item in invalid_discovery["references"]}
                    spare_image = next(image for image in images if image["id"] not in used_ids)
                    extra = copy.deepcopy(invalid_discovery["references"][0])
                    extra["asset"] = {"asset_kind":"generated_image", "asset_id":spare_image["id"], "image_path":spare_image["file_path"]}
                    invalid_discovery["references"].append(extra)
                    call("save_manga_pipeline_artifact", {"project_id":current_id, "chapter_id":reference_chapter,
                        "stage":"reference_discovery", "page_number":1, "payload":invalid_discovery,
                        "expected_prompt_updated_at":reference["updated_at"]}, reject="最多绑定两张")
                    assert reference_page()["updated_at"] == reference["updated_at"]
                    passed("手工阶段保存与生成入口遵守同一个每目标两张上限，拒绝时不改写页面")
                    for target_kind, target_name in (("character", "林岚"), ("scene", "雨中的旧车站"), ("style", "黑白漫画光影")):
                        payload = copy.deepcopy(reference["reference_discovery"])
                        payload["requirements"].append({"target_kind": target_kind, "target_name": target_name,
                            "description": f"保持{target_name}的可见外观、空间关系与光影，不添加新事实。",
                            "usage": "作为后续页面绘制的独立视觉参考。"})
                        call("save_manga_pipeline_artifact", {"project_id": current_id, "chapter_id": reference_chapter,
                            "stage": "reference_discovery", "page_number": 1, "payload": payload,
                            "expected_prompt_updated_at": reference["updated_at"]})
                        reference = reference_page()
                        offset = len(provider_log_entries(args.provider_log))
                        handle = call("start_image_generation", reference_input(reference, target_kind=target_kind, target_name=target_name))
                        checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "typed reference image task")
                        rows = provider_log_entries(args.provider_log)[offset:]
                        assert len(rows) == 1 and "/images/" in rows[0]["path"], rows
                        if target_kind == "character":
                            assert rows[0]["path"].endswith("/images/edits") and rows[0]["body"]["has_reference_image"], rows
                        reference = reference_page()
                        bound = reference["reference_discovery"]["references"][-1]
                        assert bound["target_kind"] == target_kind and bound["target_name"] == target_name
                    passed("角色、场景和风格需求均可生成并绑定；角色请求实际携带已有立绘进入 image edit 协议")
                    before = len(provider_log_entries(args.provider_log))
                    drawing_preview = call("preview_start_manga_pipeline_request", {"project_id":current_id,
                        "chapter_number":1, "page_number":1, "run_stage":"drawing_prompt", "stop_after":"drawing_prompt"})
                    assert len(provider_log_entries(args.provider_log)) == before
                    assert "manga_page_drawing_prompt" in json.dumps(drawing_preview)
                    manga_run(current_id, run_stage="drawing_prompt", stop_after="drawing_prompt", page_number=1)
                    drawing_calls = provider_log_entries(args.provider_log)[before:]
                    assert len(drawing_calls) == 1
                    assert reference_page()["drawing_prompt"] is not None
                    passed("已保存及新生成参考图可直接预览并继续绘图提示词，只调用绘图模型，不重跑发现")
                    binding = reference["reference_bindings"][0]
                    asset_id = reference["reference_discovery"]["references"][0]["asset"]["asset_id"]
                    binding_input = {"prompt_id":reference["id"], "character_name":binding["character_name"],
                        "asset_kind":"generated_image", "asset_id":asset_id}
                    before = len(provider_log_entries(args.provider_log))
                    for action in ("add_user_image", "remove_user_image"):
                        updated = call("bind_manga_reference_image", {**binding_input,"action":action})
                        saved_binding = next(item for item in updated["reference_bindings"] if item["character_name"] == binding["character_name"])
                        assert any(item["asset_id"] == asset_id and item["asset_kind"] == "generated_image" for item in saved_binding["user_added_assets"]) == (action == "add_user_image")
                        artifacts = call("list_manga_pipeline_artifacts", {"project_id":current_id, "chapter_id":reference_chapter})
                        saved = next(item for item in artifacts if item["stage"] == "reference_bindings" and item["page_number"] == 1)
                        assert saved["payload"] == updated["reference_bindings"] == reference_page()["reference_bindings"]
                        assert next(item for item in artifacts if item["stage"] == "reference_discovery" and item["page_number"] == 1)["stale"]
                        if action == "add_user_image":
                            call("save_manga_pipeline_artifact", {"project_id":current_id, "chapter_id":reference_chapter,
                                "stage":"reference_discovery", "page_number":1, "payload":updated["reference_discovery"],
                                "expected_prompt_updated_at":updated["updated_at"]})
                    assert len(provider_log_entries(args.provider_log)) == before
                    passed("用户参考图增删同步保存页面与阶段产物，使下游过期，不调用模型")
                    discovery["references"] = []
                    discovery["requirements"].append({"target_kind":"style", "target_name":"叙述者的回忆",
                        "description":"参考图名称应按保存值原样显示。", "usage":"保留原作题名，不进行人物别名替换。"})
                    current = call("create_derived_project", {
                        "root_project_id": imported_root["id"], "derived_kind": "comic",
                        "title": "浏览器无素材参考图生成", "description": "empty image library fixture",
                        "start_chapter": 1, "end_chapter": 1,
                    })
                    current_id = current["id"]
                    report["projects"].append(current_id)
                    manga_run(current_id, run_stage="preprocessing")
                    adopt(generate(current_id))
                    manga_run(current_id, stop_after="reference_discovery")
                    reference = next(page for page in call("list_manga_page_prompts", {
                        "project_id": current_id, "chapter_id": reference_chapter,
                    }) if page["page_number"] == 1)
                    call("save_manga_pipeline_artifact", {"project_id": current_id, "chapter_id": reference_chapter,
                        "stage": "reference_discovery", "page_number": 1, "payload": discovery,
                        "expected_prompt_updated_at": reference["updated_at"]})
                report["ui_mutation_project_ids"][purpose] = current_id
            passed("浏览器写入验收使用当前有效预处理与改编计划，不复用刻意过期的导入样本")
            report["success"] = True
    except Exception as error:
        report["error"] = str(error)
        report["traceback"] = traceback.format_exc()
    secure_json(args.report, report)
    print(json.dumps({"success": report["success"], "checks": len(report["checks"]), "report": str(args.report)}))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
