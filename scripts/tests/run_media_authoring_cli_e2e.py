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

from mock_openai.fixtures.story import chapter_body_response
from run_cli_contract_e2e import checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    args = cli_e2e_parser(__doc__).parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "projects": [], "generation_coverage": []}
    try:
        with tempfile.TemporaryDirectory(prefix="media-authoring-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            secure_json(work / "runtime.json", runtime_snapshot(args.provider_url))
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

            def passed(message, generation_coverage=()):
                report["checks"].append(message)
                report["generation_coverage"].extend(generation_coverage)
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
                return call("generate_media_adaptation_plan", {
                    "project_id": project_id, "chapter_id": chapter_id, "task_id": str(uuid.uuid4()),
                    "instruction": "保留原作因果，以本媒介独立改编", "video_reference": reference,
                    "clear_video_reference": False, "record_id": base["id"] if base else None,
                    "expected_revision": base["revision"] if base else None, "resume_from_checkpoint": True,
                })

            root = checked(cli, ["project", "create", "--title", "派生媒体独立创作验收", "--project-type", "novel_long", "--use-context"], "create root")[0]["data"]
            chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "旧车站", "--content", chapter_body_response()], "import source")[0]["data"]
            root_id, chapter_id = root["id"], chapter["id"]
            report.update(root_id=root_id, chapter_id=chapter_id)
            for name in ("林岚", "周衡"):
                call("create_profile", {"project_id": root_id, "name": name, "base_attributes": {"explicit_attributes": {}, "implicit_attributes": {}, "social_attributes": {}}})
            scene_result = call("create_scene_profile", {"project_id": root_id, "profile": {"name": "旧车站通道", "scene_type": "room", "importance": "primary", "base_description": "封闭通道",
                "visual_description": "木门、挂钟与清晰的通道", "spatial_layout": "通道通向机房", "atmosphere": "安静", "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": "旧车站通道"}})
            scene_profile_id = scene_result["profile"]["id"]
            object_result = call("create_object_profile", {"project_id": root_id, "profile": {
                "name": "旧怀表", "object_type": "token", "importance": "primary",
                "visual_description": "旧式铜壳怀表，表针逆行。", "material_and_state": "磨损铜壳，秒针逆行。",
                "story_usage": "证明时间异常发生。", "visual_usage": "作为林岚手部动作的连续性参照。",
                "generation_usage": "保留旧式铜壳与逆行指针。", "first_seen_chapter": 1,
                "last_seen_chapter": 1, "image_prompt_seed": "林岚掌心中的旧怀表",
            }})
            object_profile_id = object_result["profile"]["id"]

            def derive(kind):
                project = call("create_derived_project", {"root_project_id": root_id, "derived_kind": kind, "title": "独立" + kind, "description": "isolated stage fixture", "start_chapter": 1, "end_chapter": 1})
                report["projects"].append(project["id"])
                return project["id"]

            comic, game = derive("comic"), derive("galgame")
            assert call("list_video_adaptation_references", {"project_id": comic, "chapter_id": chapter_id}) == []
            text_only_provider_start = len(provider_log_entries(args.provider_log))
            comic_adaptation = adopt(generate(comic))
            game_adaptation = adopt(generate(game))
            assert comic_adaptation["document"]["target"] == "manga" and game_adaptation["document"]["target"] == "galgame"
            assert comic_adaptation["inputs"]["video_reference"] is None
            passed("漫画和 Galgame 在没有视频项目或拍摄剧本时独立生成并采用改编计划")

            def manga_run(stop_after):
                handle = call("start_manga_pipeline", {
                    "meta": {"start_request_id": str(uuid.uuid4())},
                    "project_id": comic,
                    "chapter_number": 1,
                    "stop_after": stop_after,
                })
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    status = call("get_manga_pipeline_status", {"project_id": comic, "chapter_id": chapter_id})
                    task = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "manga task")[0]["data"]
                    if status.get("task_handle"):
                        assert status["task_handle"]["task_id"] == handle["task_id"], status
                    if task["handle"]["status"] in {"completed", "failed", "cancelled", "interrupted"}:
                        assert task["handle"]["status"] == "completed", task
                        if status["pipeline_status"] == "completed":
                            return
                    time.sleep(.1)
                raise AssertionError("漫画阶段没有结束")

            manga_run("page_slots")
            pages = call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id})
            assert len(pages) == 2
            assert all(page["page_content"] and page["page_plan"] is None and page["drawing_prompt"] is None for page in pages)
            before = len(llm_calls())
            manga_run("drawing_prompt")
            pages = call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id})
            assert len(llm_calls()) - before == len(pages) * 3, "续传不能重复抽取或页面拆分模型调用"
            assert all(page["page_adaptation"] and page["page_plan"] and page["drawing_prompt"] for page in pages)
            assert pages[0]["source"]["adaptation_plan_id"] == comic_adaptation["id"]
            assert all(page["generated_image_id"] is None for page in pages)
            passed("漫画先独立确定页面数量与内容；续传后每页依次完成改编、分镜和绘图提示词")

            game_input = {"project_id": game, "task_id": str(uuid.uuid4()), "resume_from_checkpoint": True, "start_from_stage": None, "stop_after": "branch_design"}
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
            assert not any(
                "/images/" in row["path"] or "/videos" in row["path"]
                for row in provider_log_entries(args.provider_log)[text_only_provider_start:]
            )
            passed("Galgame 分支阶段单独停止并保留人工选择；剧本使用本地单元；一键文本流程不生成付费媒体")

            def generate_image(project_id, operation, expected_metadata):
                provider_start = len(provider_log_entries(args.provider_log))
                llm_start = len(llm_calls())
                handle = call("start_image_generation", {
                    "project_id": project_id,
                    "meta": {"start_request_id": str(uuid.uuid4())},
                    "operation": operation,
                })
                checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "image task")
                task = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "image task result")[0]["data"]
                assert task["handle"]["status"] == "completed", task
                result_ref = task["result_ref"]
                assert result_ref["kind"] == "generated_image" and result_ref["revision"], task
                snapshot = checked(cli, ["task", "snapshot", handle["task_id"], "--attempt", str(handle["attempt"])], "image task snapshot")[0]["data"]
                terminal = snapshot["terminal_event"]
                assert terminal["event"]["kind"] == "completed", terminal
                assert terminal["event"]["data"]["result_ref"] == result_ref, terminal
                images = call("list_images_by_project", {"project_id": project_id})
                image = next(row for row in images if str(row["id"]) == result_ref["id"])
                metadata = image.get("metadata_json") or {}
                for key, value in expected_metadata.items():
                    assert metadata.get(key) == value, (key, metadata, value)
                provider_calls = [
                    row for row in provider_log_entries(args.provider_log)[provider_start:]
                    if row["path"].endswith(("/images/generations", "/images/edits"))
                ]
                assert len(provider_calls) == 1, provider_calls
                assert isinstance(provider_calls[0]["body"].get("prompt"), str) and provider_calls[0]["body"]["prompt"].strip()
                assert len(llm_calls()) == llm_start, "image operation unexpectedly called an LLM"
                return image

            object_image = generate_image(root_id, {
                "kind": "object_profile", "object_profile_id": object_profile_id,
                "chapter_id": chapter_id, "config_id": 900002, "width": 1024, "height": 1024,
            }, {"profile_kind": "object", "profile_id": object_profile_id})
            scene_image = generate_image(root_id, {
                "kind": "scene_profile", "scene_profile_id": scene_profile_id,
                "chapter_id": chapter_id, "config_id": 900002, "width": 1024, "height": 1024,
            }, {"profile_kind": "scene", "profile_id": scene_profile_id})
            passed(
                "对象与场景资料分别经统一图片任务生成，并从正式图片记录读回",
                ("image.object_profile", "image.scene_profile"),
            )

            page_image = generate_image(comic, {
                "kind": "page", "chapter_number": 1, "page_number": 1,
                "width": 1024, "height": 1024,
            }, {"page_number": 1})
            before_attached_images = len(provider_log_entries(args.provider_log))
            manga_run(None)
            pages = call("list_manga_page_prompts", {"project_id": comic, "chapter_id": chapter_id})
            assert len(pages) == 2 and all(isinstance(page.get("generated_image_id"), int) for page in pages), pages
            page_images = call("list_images_by_project", {"project_id": comic})
            page_image_ids = {image["id"] for image in page_images}
            assert all(page["generated_image_id"] in page_image_ids for page in pages), pages
            attached_calls = [
                row for row in provider_log_entries(args.provider_log)[before_attached_images:]
                if row["path"].endswith(("/images/generations", "/images/edits"))
            ]
            assert attached_calls, "Manga pipeline did not reach attached page-image Provider generation"
            assert page_image["id"] in page_image_ids
            passed(
                "漫画单页任务与 Manga 工作流附属图片路径都完成 Provider 调用和图片/页面读回",
                ("image.page", "manga.attached_page_image"),
            )

            copies = {}
            for project_id in (comic, game):
                copied = call("duplicate_project", {"project_id": project_id, "new_title": "保留全部计划的副本"})
                copies[project_id] = copied["id"]
                original_workspace = workspace(project_id)
                copied_workspace = workspace(copied["id"])
                original_docs = original_workspace["documents"]
                copied_docs = copied_workspace["documents"]
                assert len(copied_docs) == len(original_docs)
                assert set(view["record"]["id"] for view in original_docs).isdisjoint(view["record"]["id"] for view in copied_docs)
                original_artifact_state = sorted(
                    (artifact["kind"], artifact["count"], artifact["stale"])
                    for artifact in original_workspace["artifacts"]
                )
                copied_artifact_state = sorted(
                    (artifact["kind"], artifact["count"], artifact["stale"])
                    for artifact in copied_workspace["artifacts"]
                )
                assert copied_artifact_state == original_artifact_state
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
            report["success"] = True
    except Exception as error:
        report["error"] = str(error)
        report["traceback"] = traceback.format_exc()
    secure_json(args.report, report)
    print(json.dumps({"success": report["success"], "checks": len(report["checks"]), "report": str(args.report)}))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
