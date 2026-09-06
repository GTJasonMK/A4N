#!/usr/bin/env python3
"""Focused regressions for CLI contract coverage and provider evidence."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from mock_openai_provider import chapter_body_response
from run_cli_contract_e2e import (
    classify,
    completion_flags,
    provider_log_summary,
    scenario_execution_success,
)
from run_all_workflows_e2e import (
    PROBE_ID,
    PROBE_TASK_ID,
    PROBE_WORKFLOW_ID,
    _replace_fixture_ids,
    _replace_fixture_values,
    _provider_delta,
    _provider_summary,
    auth_invocation,
    classify as manifest_classify,
    default_args,
    fixture_payload,
    fixture_path_coverage,
    fixture_result_classification,
    manifest_data,
    missing_fixture_provider_paths,
    ordered_descriptors,
    output_parts,
    runtime_snapshot_for_fixture,
    redact,
    safe_probe_payload,
    seed_fixtures,
    _first_video_segment_target,
    validate_auth_cli_result,
    validate_result_schema,
)


class CliContractRunnerTest(unittest.TestCase):
    def test_classify_reports_uncovered_stale_and_conflicting_entries(self) -> None:
        coverage = classify(
            {
                "commands": [
                    {"name": "covered", "domain": "test"},
                    {"name": "conflicted", "domain": "test"},
                    {"name": "uncovered", "domain": "test"},
                ]
            },
            {
                "variables": {},
                "scenarios": [
                    {"name": "covered scenario", "command": "covered", "args": {}},
                    {"name": "conflicting scenario", "command": "conflicted", "args": {}},
                    {"name": "stale scenario", "command": "stale_scenario", "args": {}},
                ],
                "skips": {
                    "conflicted": "must not be both scenario and skip",
                    "stale_skip": "command was removed",
                },
            },
        )

        self.assertEqual(coverage["uncovered"], ["uncovered"])
        self.assertEqual(coverage["conflicts"], ["conflicted"])
        self.assertEqual(
            coverage["stale"],
            [
                {
                    "kind": "scenario",
                    "name": "stale scenario",
                    "command": "stale_scenario",
                },
                {"kind": "skip", "name": "stale_skip", "command": "stale_skip"},
            ],
        )
        self.assertEqual(
            coverage["counts"],
            {
                "total": 3,
                "scenario": 1,
                "skipped": 0,
                "uncovered": 1,
                "conflict": 1,
                "stale": 2,
            },
        )
        self.assertEqual(completion_flags(coverage), (False, False))

    def test_skip_does_not_count_as_tested_completion(self) -> None:
        manifest = {
            "commands": [
                {"name": "covered", "domain": "test"},
                {"name": "skipped", "domain": "test"},
            ]
        }
        coverage = classify(
            manifest,
            {
                "variables": {},
                "scenarios": [{"name": "covered", "command": "covered", "args": {}}],
                "skips": {"skipped": "requires an external system"},
            },
        )

        self.assertEqual(completion_flags(coverage), (True, False))

        fully_scenario_covered = classify(
            manifest,
            {
                "variables": {},
                "scenarios": [
                    {"name": "covered", "command": "covered", "args": {}},
                    {"name": "formerly skipped", "command": "skipped", "args": {}},
                ],
                "skips": {},
            },
        )
        self.assertEqual(completion_flags(fully_scenario_covered), (True, True))

    def test_partial_business_ledger_can_pass_without_claiming_manifest_complete(self) -> None:
        self.assertTrue(scenario_execution_success(None, [{"status": "passed"}]))
        self.assertFalse(scenario_execution_success(None, []))
        self.assertFalse(scenario_execution_success(None, [{"status": "failed"}]))
        self.assertFalse(scenario_execution_success("provider failed", [{"status": "passed"}]))

    def test_provider_log_separates_preflight_and_scenarios(self) -> None:
        entries = [
            {"path": "/v1/chat/completions"},
            {"path": "/v1/images/generations"},
            {"path": "/v1/chat/completions"},
            {"path": "/v1/videos/generations"},
        ]
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "requests.jsonl"
            path.write_text(
                "".join(json.dumps(entry) + "\n" for entry in entries),
                encoding="utf-8",
            )
            summary = provider_log_summary(path, scenario_start_index=2)

        self.assertEqual(summary["requests"], 4)
        self.assertEqual(
            summary["preflight"],
            {
                "requests": 2,
                "paths": {
                    "/v1/chat/completions": 1,
                    "/v1/images/generations": 1,
                },
            },
        )
        self.assertEqual(
            summary["scenarios"],
            {
                "requests": 2,
                "paths": {
                    "/v1/chat/completions": 1,
                    "/v1/videos/generations": 1,
                },
            },
        )

    def test_manifest_probe_resolves_nested_defs_without_breaking_the_schema(self) -> None:
        descriptor = {
            "name": "start_probe_generation",
            "effect": "write",
            "args_schema": {
                "type": "object",
                "properties": {
                    "input": {
                        "type": "object",
                        "$defs": {
                            "Meta": {
                                "type": "object",
                                "properties": {"start_request_id": {"type": "string"}},
                                "required": ["start_request_id"],
                            }
                        },
                        "properties": {
                            "meta": {"$ref": "#/$defs/Meta"},
                            "project_id": {"type": "integer"},
                        },
                        "required": ["meta", "project_id"],
                    }
                },
                "required": ["input"],
            },
        }
        payload = default_args(descriptor)
        self.assertIsInstance(payload["input"]["meta"], dict)
        self.assertEqual(payload["input"]["project_id"], 999999999)
        safe = safe_probe_payload(payload, "write", True)
        self.assertEqual(safe, payload)

    def test_schema_examples_keep_json_types_and_constraints(self) -> None:
        descriptor = {
            "name": "typed_probe",
            "args_schema": {
                "type": "object",
                "properties": {
                    "count": {"type": "integer", "minimum": 5, "maximum": 8},
                    "ratio": {"type": "number", "minimum": 0.5},
                    "enabled": {"type": "boolean"},
                    "kind": {"const": "fixture", "type": "string"},
                    "payload": True,
                    "token": {"type": "string", "minLength": 8, "pattern": "^[A-Za-z0-9_.:-]+$"},
                },
                "required": ["count", "ratio", "enabled", "kind", "payload", "token"],
            },
        }
        payload = default_args(descriptor)
        self.assertEqual(payload["count"], 5)
        self.assertGreaterEqual(payload["ratio"], 0.5)
        self.assertIs(payload["enabled"], False)
        self.assertEqual(payload["kind"], "fixture")
        self.assertIsInstance(payload["payload"], dict)
        self.assertGreaterEqual(len(payload["token"]), 8)

    def test_read_only_probe_does_not_execute_a_write_payload(self) -> None:
        payload = {"input": {"title": "would mutate"}}
        self.assertEqual(safe_probe_payload(payload, "write", False), {})
        self.assertEqual(safe_probe_payload(payload, "read", False), payload)

    def test_success_result_is_checked_against_required_fields(self) -> None:
        schema = {
            "type": "object",
            "properties": {"id": {"type": "integer"}, "name": {"type": "string"}},
            "required": ["id", "name"],
        }
        validate_result_schema(schema, {"id": 1, "name": "fixture"}, schema)
        with self.assertRaises(ValueError):
            validate_result_schema(schema, {"id": 1}, schema)

    def test_result_schema_accepts_null_union_branch(self) -> None:
        schema = {
            "anyOf": [
                {"type": "object", "required": ["id"]},
                {"type": "null"},
            ]
        }
        validate_result_schema(schema, None, schema)
        validate_result_schema(schema, {"id": 1}, schema)

    def test_manifest_probe_classifies_expected_http_errors_and_real_failures(self) -> None:
        self.assertEqual(manifest_classify({"ok": True}), "business_success")
        self.assertEqual(
            manifest_classify({
                "ok": False,
                "exit_code": 8,
                "error": {"code": "not_found", "http_status": 404, "message": "missing"},
            }),
            "expected_state_rejection",
        )
        self.assertEqual(
            manifest_classify({
                "ok": False,
                "exit_code": 8,
                "error": {"code": "forbidden", "http_status": 403, "message": "denied"},
            }),
            "permission_rejection",
        )
        self.assertEqual(
            manifest_classify({
                "ok": False,
                "exit_code": 8,
                "error": {"code": "internal_server_error", "http_status": 500, "message": "database error"},
            }),
            "real_failure",
        )

    def test_manifest_payload_redaction_hides_auth_material(self) -> None:
        self.assertEqual(
            redact({"password": "secret", "nested": {"api_key": "key", "count": 1}}),
            {"password": "[REDACTED]", "nested": {"api_key": "[REDACTED]", "count": 1}},
        )
        self.assertEqual(manifest_data({"data": {"commands": [{"name": "health_check"}]}})["commands"][0]["name"], "health_check")

    def test_register_probe_preserves_the_cli_invite_code_contract(self) -> None:
        parts, stdin = auth_invocation(
            "register_user",
            {
                "input": {
                    "username": "probe",
                    "email": "probe@example.invalid",
                    "invite_code": "invite",
                    "password": "manifest-probe-password",
                }
            },
            False,
        )
        self.assertIn("--invite-code", parts)
        self.assertEqual(parts[parts.index("--invite-code") + 1], "invite")
        self.assertEqual(stdin, "manifest-probe-password\n")

    def test_fixture_writes_keep_provider_anchors_and_named_materials(self) -> None:
        state = {"project_id": 41, "chapter_id": 42}
        for command in ("import_chapter", "update_chapter_content"):
            payload = fixture_payload(
                command,
                {"input": {"project_id": 41, "chapter_id": 42, "content": "probe"}},
                state,
                "manifest-test",
                "http://127.0.0.1:1/v1",
                True,
            )
            self.assertEqual(payload["input"]["content"], chapter_body_response())

        object_payload = fixture_payload(
            "create_object_profile",
            {"input": {"profile": {}}},
            state,
            "manifest-test",
            "http://127.0.0.1:1/v1",
            True,
        )
        self.assertTrue(object_payload["input"]["profile"]["name"].startswith("齿轮暗记-"))

        asset_payload = fixture_payload(
            "run_galgame_asset_generation",
            {"input": {}},
            {"galgame_project_id": 43, "image_config_id": 44, "generation_spec_ids": [45, 46]},
            "manifest-test",
            "http://127.0.0.1:1/v1",
            True,
        )
        self.assertEqual(asset_payload["input"]["limit"], 1)

    def test_video_target_comes_from_the_current_segment_plan(self) -> None:
        prompt = {
            "metadata_json": {
                "segments": [
                    {
                        "segment_number": 2,
                        "source_shot_id": "shot-2",
                        "segment_index_in_shot": 1,
                    },
                    {
                        "segment_number": 1,
                        "source_shot_id": "shot-1",
                        "segment_index_in_shot": 1,
                    },
                ],
                "segment_recipes": [
                    {
                        "segment_number": 1,
                        "workflow_mode": "chat_video",
                        "prompt_output_language": "zh_cn",
                        "fixture_marker": "segment-one",
                    },
                    {
                        "segment_number": 2,
                        "workflow_mode": "chat_video",
                        "prompt_output_language": "zh_cn",
                        "fixture_marker": "segment-two",
                    },
                ],
            }
        }
        self.assertEqual(
            _first_video_segment_target(prompt),
            {
                "source_shot_id": "shot-1",
                "segment_index_in_shot": 1,
                "workflow_mode": "chat_video",
                "prompt_output_language": "zh_cn",
                "recipe": {
                    "segment_number": 1,
                    "workflow_mode": "chat_video",
                    "prompt_output_language": "zh_cn",
                    "fixture_marker": "segment-one",
                },
            },
        )
        with self.assertRaisesRegex(Exception, "可执行的目标片段"):
            _first_video_segment_target({"metadata_json": {"segments": []}})

    def test_published_video_recipe_fixture_keeps_top_level_contract(self) -> None:
        recipe = {
            "schema_version": "video_segment_execution_recipe_v1",
            "segment_number": 1,
            "workflow_mode": "boundary_frames",
            "style": "电影感",
            "prompt_output_language": "zh_cn",
            "duration_secs": 6,
            "duration_secs_options": [6],
            "aspect_ratio": "16:9",
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "guidance_mode": "none",
            "guidance_reference_image_ids": [],
            "continuity_source_video_id": None,
            "reference_media": None,
            "adopted_video_id": None,
        }
        payload = fixture_payload(
            "update_chapter_video_segment_recipe",
            {"promptId": PROBE_ID, "recipe": {"workflow_mode": "probe"}},
            {"segment_plan_prompt_id": 41, "video_segment_recipe": recipe},
            "manifest-test",
            "http://127.0.0.1:1/v1",
            True,
        )
        self.assertEqual(payload, {"promptId": 41, "recipe": recipe})

    def test_seed_records_keep_each_call_and_task_watch(self) -> None:
        command_names = [
            "create_project",
            "create_freeform_video_project",
            "import_project_text_upload",
            "accept_import_review",
            "preview_import_project_analysis_request",
            "run_import_project_analysis",
            "get_project_import_analysis_progress",
            "list_import_queue",
            "start_inspiration_generation",
            "start_blueprint_generation",
            "start_outline_generation",
            "list_parts",
            "list_chapter_outlines",
            "delete_latest_chapter_outlines",
            "start_chapter_generation",
            "list_chapters",
            "generate_chapter_analysis",
            "rag_prepare_ingest_all",
            "rag_ingest_all",
            "rag_prepare_enhanced_search",
            "rag_enhanced_search",
            "save_chapter_visual_plan_draft",
            "confirm_chapter_visual_plan",
            "get_chapter_visual_plan",
            "generate_chapter_shooting_script",
            "get_chapter_shooting_script",
            "create_profile",
            "create_object_profile",
            "create_scene_profile",
            "create_foreshadowing",
            "create_theme",
            "create_llm_config",
            "create_embedding_config",
            "add_image_config",
            "add_video_config",
            "upload_image",
            "upload_portrait",
            "upload_profile_material_image",
            "save_blueprint",
            "save_canvas_graph",
            "save_canvas_text_asset",
            "import_canvas_media",
            "create_derived_project",
            "preview_source_sync",
            "apply_source_sync_decisions",
            "duplicate_project",
            "start_galgame_pipeline",
            "get_galgame_pipeline_status",
            "get_galgame_script",
            "plan_galgame_assets",
            "run_galgame_asset_generation",
            "start_manga_pipeline",
            "get_manga_pipeline_status",
            "list_manga_page_prompts",
            "start_video_generation",
            "list_chapter_video_prompts",
            "update_chapter_video_segment_recipe",
            "list_videos_by_chapter",
            "poll_video_job",
            "sync_character_from_chapters",
            "sync_all_character_profiles",
            "sync_scene_profiles",
            "list_all_users",
            "invite_project_member",
            "list_project_invites",
            "list_my_project_invites",
        ]
        descriptors = {name: {"name": name} for name in command_names}
        task_commands = {
            "start_inspiration_generation",
            "start_blueprint_generation",
            "start_outline_generation",
            "start_chapter_generation",
            "generate_chapter_analysis",
            "start_video_generation",
            "sync_character_from_chapters",
            "sync_all_character_profiles",
            "sync_scene_profiles",
        }
        ids = {
            "create_project": {"id": 1},
            "create_freeform_video_project": {"id": 2},
            "import_project_text_upload": [{"id": 30}],
            "accept_import_review": {"id": 30, "project_id": 1},
            "list_parts": [{"id": 3}],
            "list_chapter_outlines": [{"id": 31, "updated_at": "2026-08-22T12:00:00"}],
            "delete_latest_chapter_outlines": {"warnings": []},
            "list_chapters": [{"id": 4}],
            "create_profile": {"id": 5},
            "create_object_profile": {"id": 6},
            "create_scene_profile": {"id": 7},
            "create_foreshadowing": {"id": 8},
            "create_theme": {"id": 9},
            "create_llm_config": {"id": 10},
            "create_embedding_config": {"id": 11},
            "add_image_config": {"id": 12},
            "add_video_config": {"id": 13},
            "upload_image": {"id": 14},
            "upload_portrait": {"id": 15},
            "upload_profile_material_image": {"id": 16},
            "save_canvas_graph": {"id": 17},
            "create_derived_project": {"id": 18},
            "preview_source_sync": {"run": {"id": 19}, "conflicts": []},
            "apply_source_sync_decisions": {"run": {"id": 19}, "conflicts": []},
            "duplicate_project": {"id": 20},
            "start_galgame_pipeline": {},
            "get_galgame_pipeline_status": {"checkpoints": [], "run": None},
            "get_galgame_script": {},
            "plan_galgame_assets": {"backgrounds": [{"generation_spec_id": 21}], "portrait_expressions": [], "event_cgs": [], "summary": {}},
            "run_galgame_asset_generation": {},
            "start_manga_pipeline": "manga-task",
            "get_manga_pipeline_status": {"pipeline_status": "completed", "total": 1, "completed": 1},
            "list_manga_page_prompts": [{"id": 22}],
            "list_videos_by_chapter": [{"id": 23}],
            "poll_video_job": {"id": 23, "status": "completed"},
            "save_canvas_text_asset": {"id": 24},
            "import_canvas_media": {},
            "get_project_import_analysis_progress": None,
            "run_import_project_analysis": {"progress": {}, "analyzed_chapters": 1},
            "preview_import_project_analysis_request": {},
            "list_import_queue": [],
            "get_chapter_visual_plan": {},
            "generate_chapter_shooting_script": {"script": {}},
            "get_chapter_shooting_script": {"script": {}},
        }
        calls: list[str] = []
        payloads: list[tuple[str, dict]] = []
        destructive_calls: list[tuple[str, bool]] = []
        task_counter = 0

        def fake_invoke(_cli, command, payload, allow_destructive, _descriptor):
            nonlocal task_counter
            calls.append(command)
            payloads.append((command, payload))
            destructive_calls.append((command, allow_destructive))
            if command == "save_chapter_visual_plan_draft":
                data = {
                    "record": {"id": 25, "revision": 1},
                    "plan": {},
                }
            elif command in task_commands:
                task_counter += 1
                data = {"task_id": f"00000000-0000-4000-8000-{task_counter:012d}"}
            elif command == "confirm_chapter_visual_plan":
                data = {"visual_plan": {"record": {"id": 25, "revision": 1}, "plan": {}}, "invalidated_project_ids": []}
            elif command == "start_video_generation":
                task_counter += 1
                data = {"task_id": f"00000000-0000-4000-8000-{task_counter:012d}"}
            elif command == "list_chapter_video_prompts":
                data = [
                    {
                        "id": 26,
                        "workflow_mode": "shared_segment_plan",
                        "prompt_type": "frame_segment_plan",
                        "metadata_json": {
                            "segments": [
                                {
                                    "segment_number": 1,
                                    "source_shot_id": "shot-1",
                                    "segment_index_in_shot": 1,
                                }
                            ],
                            "segment_recipes": [
                                {
                                    "segment_number": 1,
                                    "workflow_mode": "chat_video",
                                    "prompt_output_language": "zh_cn",
                                }
                            ],
                        },
                    },
                    {"id": 27, "workflow_mode": "boundary_frames", "prompt_type": "video_prompt"},
                ]
            elif command == "list_all_users":
                data = [{"username": "admin"}, {"username": "collaborator"}]
            elif command == "list_project_invites":
                data = [{"id": 28}]
            else:
                data = ids.get(command, {})
            return {"ok": True, "classification": "business_success", "duration_ms": 1, "data": data}

        def fake_watch(_cli, task_id, records, label):
            records.append({"command": "task watch", "task_id": task_id, "label": label})

        with patch("run_all_workflows_e2e.invoke", side_effect=fake_invoke), patch(
            "run_all_workflows_e2e._watch_seed_task", side_effect=fake_watch
        ):
            state, records = seed_fixtures(
                object(),
                descriptors,
                "manifest-test",
                "http://127.0.0.1:1/v1",
                allow_destructive=True,
                current_username="admin",
            )

        self.assertEqual(state["project_id"], 1)
        self.assertEqual(state["import_review_id"], 30)
        self.assertEqual(state["graph_id"], 17)
        self.assertEqual(len(calls), len(records) - sum(record["command"] == "task watch" for record in records))
        self.assertEqual(records[0]["command"], "create_project")
        self.assertIn("save_canvas_graph", calls)
        self.assertIn("save_chapter_visual_plan_draft", calls)
        self.assertIn("confirm_chapter_visual_plan", calls)
        self.assertIn("apply_source_sync_decisions", calls)
        coverage = fixture_path_coverage(records)
        self.assertTrue(coverage["complete"], coverage)
        video_prompt_calls = [payload for command, payload in payloads if command == "start_video_generation"]
        self.assertEqual(video_prompt_calls[1]["input"]["operation"]["kind"], "prompt_frames")
        self.assertEqual(video_prompt_calls[1]["input"]["operation"]["options"]["workflow_mode"], "boundary_frames")
        self.assertEqual(video_prompt_calls[2]["input"]["operation"]["kind"], "prompt_videos")
        self.assertEqual(video_prompt_calls[2]["input"]["operation"]["options"]["workflow_mode"], "boundary_frames")
        self.assertEqual(video_prompt_calls[2]["input"]["operation"]["options"]["prompt_output_language"], "zh_cn")
        self.assertEqual(
            [record["label"] for record in records if record["command"] == "task watch"],
            [
                "start_inspiration_generation",
                "start_blueprint_generation",
                "start_outline_generation",
                "start_outline_generation",
                "start_chapter_generation",
                "start_video_generation",
                "start_video_generation",
                "start_video_generation",
                "start_video_generation",
            ],
        )
        self.assertLess(calls.index("import_project_text_upload"), calls.index("accept_import_review"))
        self.assertLess(calls.index("accept_import_review"), calls.index("preview_import_project_analysis_request"))
        self.assertEqual(
            {command for command, allowed in destructive_calls if allowed},
            {
                "confirm_chapter_visual_plan",
                "apply_source_sync_decisions",
                "delete_latest_chapter_outlines",
            },
        )
        self.assertTrue(dict(destructive_calls)["confirm_chapter_visual_plan"])
        self.assertTrue(dict(destructive_calls)["apply_source_sync_decisions"])
        self.assertTrue(dict(destructive_calls)["delete_latest_chapter_outlines"])

    def test_fixture_path_coverage_requires_successful_commands(self) -> None:
        records = [
            {"command": command, "ok": True}
            for command in {
                "create_project",
                "import_project_text_upload",
                "accept_import_review",
                "start_inspiration_generation",
                "start_blueprint_generation",
                "start_outline_generation",
                "list_parts",
                "list_chapter_outlines",
                "start_chapter_generation",
                "list_chapters",
            }
        ]
        coverage = fixture_path_coverage(records)
        self.assertFalse(coverage["complete"])
        self.assertEqual(coverage["paths"]["novel_authoring"]["status"], "passed")
        self.assertEqual(coverage["paths"]["rag"]["status"], "failed")

        records.extend(
            {"command": command, "ok": True}
            for path in coverage["paths"].values()
            for command in path["missing_commands"]
        )
        complete = fixture_path_coverage(records)
        self.assertTrue(complete["complete"])
        with_collaborator = fixture_path_coverage(
            records + [{"command": "accept_project_invite", "ok": True}],
            collaborator_enabled=True,
        )
        self.assertTrue(with_collaborator["complete"])

    def test_manifest_provider_summary_and_delta_preserve_phase_boundaries(self) -> None:
        entries = [
            {"path": "/preflight"},
            {"path": "/fixture-a"},
            {"path": "/fixture-b"},
            {"path": "/manifest"},
        ]
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "requests.jsonl"
            path.write_text("".join(json.dumps(entry) + "\n" for entry in entries), encoding="utf-8")
            summary = _provider_summary(path, 1, 3)
            delta = _provider_delta(path, 2)

        self.assertEqual(summary["preflight"], {"requests": 1, "paths": {"/preflight": 1}})
        self.assertEqual(summary["fixture"], {"requests": 2, "paths": {"/fixture-a": 1, "/fixture-b": 1}})
        self.assertEqual(summary["manifest"], {"requests": 1, "paths": {"/manifest": 1}})
        self.assertEqual(delta, {"requests": 2, "paths": {"/fixture-b": 1, "/manifest": 1}})

    def test_fixture_provider_routes_are_checked_separately_from_preflight(self) -> None:
        entries = [
            {"path": "/v1/chat/completions"},
            {"path": "/v1/images/generations"},
            {"path": "/v1/videos/generations"},
        ]
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "requests.jsonl"
            path.write_text(
                "".join(json.dumps(entry) + "\n" for entry in entries),
                encoding="utf-8",
            )
            self.assertEqual(missing_fixture_provider_paths(path, 0), [])
            path.write_text(
                "".join(json.dumps(entry) + "\n" for entry in entries[:2]),
                encoding="utf-8",
            )
            self.assertEqual(
                missing_fixture_provider_paths(path, 0),
                ["/v1/videos/generations"],
            )

    def test_fixture_runtime_snapshot_uses_created_config_ids(self) -> None:
        snapshot = runtime_snapshot_for_fixture(
            "http://127.0.0.1:1/v1",
            {
                "llm_config_id": 11,
                "image_config_id": 12,
                "video_config_id": 13,
                "embedding_config_id": 14,
            },
        )
        self.assertEqual(snapshot["llm"]["id"], 11)
        self.assertEqual(snapshot["llm_configs"][0]["id"], 11)
        self.assertEqual(snapshot["image"]["id"], 12)
        self.assertEqual(snapshot["video_configs"][0]["id"], 13)
        self.assertEqual(snapshot["embedding"]["id"], 14)

    def test_fixture_id_replacement_uses_the_command_specific_state(self) -> None:
        state = {
            "project_id": 11,
            "canvas_project_id": 12,
            "galgame_project_id": 13,
            "derived_project_id": 14,
            "chapter_id": 15,
            "profile_id": 16,
            "object_profile_id": 17,
            "scene_profile_id": 18,
            "foreshadowing_id": 19,
            "theme_id": 20,
            "visual_plan_id": 21,
            "visual_plan_revision": 22,
            "sync_run_id": 23,
            "invite_id": 24,
            "llm_config_id": 25,
            "embedding_config_id": 26,
            "image_config_id": 27,
            "video_config_id": 28,
            "graph_id": 29,
            "text_asset_id": 30,
            "segment_plan_prompt_id": 31,
            "video_prompt_id": 32,
            "manga_prompt_id": 33,
            "video_id": 34,
            "workflow_run_id": 35,
            "task_id": "00000000-0000-4000-8000-000000000099",
        }
        cases = [
            ("project_id", "list_projects", "project_id"),
            ("project_id", "save_canvas_graph", "canvas_project_id"),
            ("project_id", "get_galgame_script", "galgame_project_id"),
            ("derived_project_id", "preview_source_sync", "derived_project_id"),
            ("chapter_id", "list_chapters", "chapter_id"),
            ("profile_id", "get_profile", "profile_id"),
            ("object_profile_id", "get_object_profile", "object_profile_id"),
            ("scene_profile_id", "get_scene_profile", "scene_profile_id"),
            ("foreshadowing_id", "get_foreshadowing", "foreshadowing_id"),
            ("theme_id", "get_theme", "theme_id"),
            ("record_id", "get_chapter_visual_plan", "visual_plan_id"),
            ("sync_run_id", "apply_source_sync_decisions", "sync_run_id"),
            ("invite_id", "list_project_invites", "invite_id"),
            ("config_id", "check_llm_config", "llm_config_id"),
            ("config_id", "run_galgame_asset_generation", "image_config_id"),
            ("config_id", "check_embedding_config", "embedding_config_id"),
            ("config_id", "start_video_generation", "video_config_id"),
            ("graph_id", "get_canvas_graph", "graph_id"),
            ("text_asset_id", "get_canvas_text_asset", "text_asset_id"),
            ("segment_plan_prompt_id", "start_video_generation", "segment_plan_prompt_id"),
            ("prompt_id", "list_manga_page_prompts", "manga_prompt_id"),
            ("prompt_id", "start_video_generation", "video_prompt_id"),
            ("video_id", "poll_video_job", "video_id"),
            ("workflow_run_id", "workflow_show", "workflow_run_id"),
            ("task_id", "task_watch", "task_id"),
        ]
        for key, command, state_key in cases:
            probe = (
                PROBE_TASK_ID
                if key == "task_id"
                else PROBE_WORKFLOW_ID
                if key == "workflow_run_id"
                else PROBE_ID
            )
            self.assertEqual(
                _replace_fixture_ids(probe, key, state, command),
                state[state_key],
                (key, command),
            )

        nested = _replace_fixture_values(
            {"input": {"project_id": PROBE_ID, "ids": [{"video_id": PROBE_ID}]}},
            state,
            "list_videos_by_chapter",
        )
        self.assertEqual(
            nested,
            {"input": {"project_id": 11, "ids": [{"video_id": 34}]}},
        )

    def test_fixture_failure_is_a_real_failure_even_for_expected_state_error(self) -> None:
        self.assertEqual(
            fixture_result_classification(
                {
                    "ok": False,
                    "classification": "expected_state_rejection",
                }
            ),
            "real_failure",
        )
        self.assertEqual(
            fixture_result_classification(
                {"ok": True, "classification": "business_success"}
            ),
            "business_success",
        )

    def test_manifest_descriptor_order_and_file_outputs_are_stable(self) -> None:
        descriptors = [
            {"name": "read_z", "effect": "read", "execution": "immediate"},
            {"name": "write_a", "effect": "write", "execution": "immediate"},
            {"name": "task_a", "effect": "read", "execution": "task"},
            {"name": "delete_a", "effect": "destructive", "execution": "immediate"},
        ]
        self.assertEqual(
            [item["name"] for item in ordered_descriptors(descriptors)],
            ["write_a", "task_a", "read_z", "delete_a"],
        )
        with tempfile.TemporaryDirectory() as temporary:
            text_parts, text_path = output_parts("export_txt_data", {"file_outputs": ["/data"]}, Path(temporary))
            binary_parts, binary_path = output_parts("export_epub_data", {"file_outputs": ["/data"]}, Path(temporary))
        self.assertEqual(text_parts[0], "--save-text")
        self.assertEqual(text_path.suffix, ".txt")
        self.assertEqual(binary_parts[0], "--save-base64")
        self.assertEqual(binary_path.suffix, ".bin")

    def test_auth_cli_result_and_destructive_success_use_their_own_contracts(self) -> None:
        validate_auth_cli_result("register_user", {"server": "x", "user": {}, "session": {}})
        with self.assertRaises(ValueError):
            validate_auth_cli_result("register_user", {"server": "x", "user": {}})
        self.assertEqual(
            manifest_classify({"ok": True}, {"effect": "destructive"}),
            "destructive_success",
        )


if __name__ == "__main__":
    unittest.main()
