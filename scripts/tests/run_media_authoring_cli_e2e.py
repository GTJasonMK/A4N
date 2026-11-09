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

            def preprocessing_model(name, max_concurrent=4):
                snapshot = copy.deepcopy(mock_runtime)
                snapshot["llm"]["model"] = name
                snapshot["llm"]["max_output_tokens"] = 20000
                snapshot["llm"]["max_concurrent"] = max_concurrent
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
                **delete_input, "expected_artifact_ids": [deletion["target"]["artifact_id"]],
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
                "expected_artifact_ids": [entry["artifact_id"] for entry in [deletion["target"], *deletion["downstream"]]],
            }, destructive=True)
            before_visual_regeneration = len(llm_calls())
            manga_run(scoped_comic, stop_after="reference_discovery", page_number=2, run_stage="storyboard")
            assert len(llm_calls()) - before_visual_regeneration == 2
            passed("删除本页分镜后组合阶段重新调用分镜与参考发现，不回放已完成工作流的旧答案")

            for _ in range(2):
                deletion = call("preview_delete_manga_pipeline_artifact", delete_input)
                call("delete_manga_pipeline_artifact", {
                    **delete_input,
                    "expected_artifact_ids": [entry["artifact_id"] for entry in [deletion["target"], *deletion["downstream"]]],
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
            for purpose in ("draft", "image"):
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
