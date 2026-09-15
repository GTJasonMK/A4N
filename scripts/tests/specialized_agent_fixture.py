"""Deterministic Director decisions; IDs and versions come from real observations."""

import json


MARKER = "A4N_SPECIALIZED "
GENERAL_MARKER = "A4N_GENERAL "


def general_fixture(body, messages):
    start = next(
        (
            i
            for i in range(len(messages) - 1, -1, -1)
            if messages[i].get("role") == "user"
            and str(messages[i].get("content", "")).startswith(GENERAL_MARKER)
        ),
        None,
    )
    if start is None:
        return None
    task = json.loads(messages[start]["content"][len(GENERAL_MARKER):])
    observations = {}
    pending = set()
    for message in messages[start:]:
        for call in message.get("tool_calls") or []:
            if call["id"] in pending:
                raise ValueError("duplicate pending tool call")
            pending.add(call["id"])
        if message.get("role") == "tool":
            call_id = message["tool_call_id"]
            if call_id not in pending:
                raise ValueError("orphan tool observation")
            pending.remove(call_id)
            observations[call_id] = json.loads(message["content"])
    if pending:
        raise ValueError("model was called before its tool batch was resolved")
    if any("error" in result for result in observations.values()):
        return {"content": "通用 Agent 操作未完成。"}

    def call(key, name, arguments):
        return {"tool_call": {"id": key, "name": name, "arguments": arguments}}

    def host(key, name, arguments):
        return call(key, name, {"input": arguments})

    if "search" not in observations:
        return call("search", "search_agent_tools", {"query": "get_project"})
    if "describe" not in observations:
        return call("describe", "describe_agent_tool", {"name": "get_project"})
    if "project" not in observations:
        return host("project", "get_project", {"project_id": task["project_id"]})
    return {"content": "已通过工具发现读取项目，并取得真实项目结果。"}


def director_fixture(body, messages):
    start = next((i for i in range(len(messages) - 1, -1, -1)
                  if messages[i].get("role") == "user"
                  and str(messages[i].get("content", "")).startswith(MARKER)), None)
    if start is None:
        return None
    task = json.loads(messages[start]["content"][len(MARKER):])
    observations = {}
    pending = set()
    for message in messages:
        for call in message.get("tool_calls") or []:
            if call["id"] in pending:
                raise ValueError("duplicate pending tool call")
            pending.add(call["id"])
        if message.get("role") == "tool":
            call_id = message["tool_call_id"]
            if call_id not in pending:
                raise ValueError("orphan tool observation")
            pending.remove(call_id)
    if pending:
        raise ValueError("model was called before its tool batch was resolved")
    for message in messages[start:]:
        if message.get("role") == "tool":
            observations[message["tool_call_id"]] = json.loads(message["content"])
    for result in observations.values():
        if "error" in result:
            return {"content": "操作未完成：" + str(result["error"])}

    def call(key, name, arguments):
        return {"tool_call": {"id": key, "name": name, "arguments": arguments}}

    def host(key, name, arguments):
        return call(key, name, {"input": arguments})

    mode = task.get("mode", "edit")
    visible = {tool["function"]["name"] for tool in body.get("tools", [])}
    if mode == "wait_generation":
        if "wait" not in observations:
            return call("wait", "wait_generation_task", {"taskId": task["task_id"], "attempt": task["attempt"]})
        return {"content": "已读取生成任务的实际终态。"}
    if mode == "canvas_edit":
        resource = {"project_id": task["project_id"], "graph_id": task["graph_id"]}
        if "graph" not in observations:
            return host("graph", "read_canvas_graph", {**resource, "node_ids": [task["node_id"]]})
        if "edit" not in observations:
            return host("edit", "edit_canvas_graph", {
                **resource, "expected_version": int(observations["graph"]["resource"]["revision"]),
                "operations": [{"kind": "set_node_config", "node_id": task["node_id"], "config_json": task["config"]}],
            })
        if task.get("run") and "run" not in observations:
            return call("run", "run_canvas_node", {**resource, "node_id": task["node_id"]})
        if task.get("run") and "wait" not in observations:
            return host("wait", "wait_workflow_run", {"workflow_run_id": observations["run"]["workflow_run_id"]})
        return {"content": "画布局部修改已保存，并已取得实际执行结果。" if task.get("run") else "画布局部修改已保存。"}
    if mode == "read_result":
        if "result" not in observations:
            return call("result", "read_agent_execution_result", {
                "execution_id": task["execution_id"], "pointer": "/source/content", "offset": 2, "limit": 8,
            })
        return {"content": "已读取历史工具结果：" + observations["result"]["text"]}
    if mode == "discover":
        if "describe_full" not in observations and "get_chapter_visual_plan" in visible:
            raise ValueError("full tool schema was activated before describe")
        if "search" not in observations:
            return call("search", "search_agent_tools", {"query": "visual_plan"})
        if any("args_schema" in item or "result_schema" in item for item in observations["search"]["tools"]):
            raise ValueError("search returned full schemas")
        if "describe_full" not in observations:
            return call("describe_full", "describe_agent_tool", {"name": "get_chapter_visual_plan"})
        if "get_chapter_visual_plan" not in visible:
            raise ValueError("described tool was not restored")
        if "full" not in observations:
            return host("full", "get_chapter_visual_plan", {
                "project_id": task["project_id"], "source_chapter_id": task["chapter_id"],
            })
        handle = observations["full"]
        if handle.get("complete") is not False or handle.get("read_tool") != "read_agent_execution_result":
            raise ValueError("large result did not return a durable read handle")
        if "slice" not in observations:
            return call("slice", "read_agent_execution_result", {
                "execution_id": handle["execution_id"], "pointer": "/source/content", "offset": 2, "limit": 8,
            })
        return {"content": "已按需读取大结果片段：" + observations["slice"]["text"]}

    if mode == "generate":
        resource = {
            "project_id": task["project_id"],
            "source_chapter_id": task["chapter_id"],
        }
        if "shot" not in observations:
            return host(
                "shot",
                "read_chapter_visual_plan",
                {**resource, "view": {"kind": "segment", "segment_id": task["shot_id"]}},
            )
        plan_resource = observations["shot"]["resource"]
        if "start" not in observations:
            return host(
                "start",
                "start_visual_plan_prompt_generation",
                {
                    **resource,
                    "record_id": int(plan_resource["id"]),
                    "expected_revision": int(plan_resource["revision"]),
                    "meta": {"start_request_id": task.get("start_request_id", "director-agent-generate")},
                    "segment_id": task["shot_id"],
                    "user_description": task.get("prompt", "镜头缓慢推近，保留门把手停顿和冷光转场。"),
                },
            )
        handle = observations["start"]
        if "wait" not in observations:
            return call("wait", "wait_generation_task", {"taskId": handle["task_id"], "attempt": handle["attempt"]})
        summary = observations["wait"]
        result_ref = summary.get("result_ref")
        if not isinstance(result_ref, dict) or not result_ref.get("id"):
            raise ValueError("visual prompt generation did not return a result reference")
        if "candidate" not in observations:
            return host(
                "candidate",
                "read_visual_plan_prompt_candidate",
                {**resource, "generation_run_id": int(result_ref["id"])},
            )
        candidate = observations["candidate"]
        if not candidate.get("prompt") or candidate.get("plan_sha256") != plan_resource.get("hash"):
            raise ValueError("visual prompt candidate is not bound to the observed plan")
        return {"content": "已读取镜头提示词候选，候选仍等待用户采用。"}

    if mode == "image_generate":
        resource = {
            "project_id": task["project_id"],
            "source_chapter_id": task["chapter_id"],
        }
        if "point" not in observations:
            return host(
                "point",
                "read_chapter_visual_plan",
                {**resource, "view": {"kind": "point", "point_id": task["point_id"]}},
            )
        plan_resource = observations["point"]["resource"]
        options = {
            "source_chapter_id": task["chapter_id"],
            "record_id": int(plan_resource["id"]),
            "expected_revision": int(plan_resource["revision"]),
            "point_id": task["point_id"],
            "user_description": task.get("prompt", "沿用当前构图，保留人物和冷光方向。"),
            "config_id": None,
        }
        if "preview" not in observations:
            return host("preview", "preview_visual_plan_point_generation", {**resource, "options": options})
        preview = observations["preview"]
        if not preview.get("aspect_ratio") or not isinstance(preview.get("reference_image_ids"), list):
            raise ValueError("image generation preview is incomplete")
        if "start" not in observations:
            return host(
                "start",
                "start_image_generation",
                {
                    "project_id": task["project_id"],
                    "meta": {"start_request_id": task.get("start_request_id", "director-agent-image")},
                    "operation": {"kind": "visual_plan_point", "options": options},
                },
            )
        handle = observations["start"]
        if "wait" not in observations:
            return call("wait", "wait_generation_task", {"taskId": handle["task_id"], "attempt": handle["attempt"]})
        if "images" not in observations:
            return host("images", "list_images_by_projects", {"project_ids": [task["project_id"]]})
        image = next(
            (
                item
                for item in observations["images"]
                if isinstance(item, dict)
                and isinstance(item.get("metadata_json"), dict)
                and item["metadata_json"].get("director_candidate", {}).get("point_id") == task["point_id"]
            ),
            None,
        )
        if image is None:
            raise ValueError("image generation did not publish a director candidate")
        return {"content": "已读取图片生成候选，图片点仍等待用户显式绑定。"}

    if mode == "compose":
        project_id, chapter_id = task["project_id"], task["chapter_id"]
        resource = {"project_id": project_id, "source_chapter_id": chapter_id}

        # Full-plan reads and draft saves are intentionally activated through
        # discovery before the Agent can compose a timeline from the starter.
        if "get_chapter_visual_plan" not in visible:
            if "search" not in observations:
                return call("search", "search_agent_tools", {"query": "visual_plan"})
            if "describe_get" not in observations:
                return call("describe_get", "describe_agent_tool", {"name": "get_chapter_visual_plan"})
        if "plan" not in observations:
            return host("plan", "get_chapter_visual_plan", resource)

        if "save_chapter_visual_plan_draft" not in visible:
            if "describe_save" not in observations:
                return call(
                    "describe_save",
                    "describe_agent_tool",
                    {"name": "save_chapter_visual_plan_draft"},
                )
        starter = observations["plan"].get("starter_plan")
        if not isinstance(starter, dict):
            raise ValueError("get_chapter_visual_plan did not return starter_plan")
        if "save" not in observations:
            return host(
                "save",
                "save_chapter_visual_plan_draft",
                {
                    **resource,
                    "authoring_mode": "collaborative",
                    "record_id": None,
                    "expected_revision": None,
                    "plan": starter,
                },
            )

        def current_resource(key):
            value = observations[key]
            resource_value = value.get("resource")
            if resource_value:
                return resource_value
            record = value.get("record")
            if not isinstance(record, dict):
                raise ValueError(f"{key} did not return a plan record")
            return {"id": str(record["id"]), "revision": str(record["revision"])}

        if "outline" not in observations:
            saved = current_resource("save")
            return host(
                "outline",
                "read_chapter_visual_plan",
                {
                    **resource,
                    "record_id": int(saved["id"]),
                    "expected_revision": int(saved["revision"]),
                    "view": {"kind": "outline", "offset": 0, "limit": 20},
                },
            )

        def edit(key, previous, operation):
            if key in observations:
                return None
            current = current_resource(previous)
            return host(
                key,
                "edit_chapter_visual_plan_draft",
                {
                    **resource,
                    "record_id": int(current["id"]),
                    "expected_revision": int(current["revision"]),
                    "operation": operation,
                },
            )

        insert = edit(
            "insert_door",
            "outline",
            {
                "kind": "insert_point",
                "point": {
                    "id": "door",
                    "at_ms": 2500,
                    "label": "门把手",
                    "image_id": None,
                    "description": "手停在旧门把手上",
                    "reference_image_ids": [],
                },
            },
        )
        if insert:
            return insert

        inserted_plan = observations["insert_door"]["visual_plan"]["plan"]
        door = next(point for point in inserted_plan["points"] if point["id"] == "door")
        update = edit(
            "update_door",
            "insert_door",
            {
                "kind": "update_point",
                "point": {
                    **door,
                    "image_id": task["image_id"],
                    "reference_image_ids": [task["image_id"]],
                    "description": "手停在旧门把手上，画面保留雨水反光",
                },
            },
        )
        if update:
            return update

        split_first = edit(
            "split_first",
            "update_door",
            {
                "kind": "split_segment",
                "segment_id": "shot-1",
                "point_id": "door",
                "right_segment_id": "shot-2",
                "left_prompt": "林岚在旧车站通道观察挂钟，手停在门把手上。",
                "right_prompt": "门被推开，林岚准备进入地下机房。",
                "left_scene_binding": {"kind": "scene", "scene_profile_id": task["scene_a_id"]},
                "right_scene_binding": {"kind": "scene", "scene_profile_id": task["scene_a_id"]},
            },
        )
        if split_first:
            return split_first

        insert_enter = edit(
            "insert_enter",
            "split_first",
            {
                "kind": "insert_point",
                "point": {
                    "id": "enter",
                    "at_ms": 7500,
                    "label": "进入机房",
                    "image_id": None,
                    "description": "门后显出地下机房的冷光",
                    "reference_image_ids": [],
                },
            },
        )
        if insert_enter:
            return insert_enter

        split_second = edit(
            "split_second",
            "insert_enter",
            {
                "kind": "split_segment",
                "segment_id": "shot-2",
                "point_id": "enter",
                "right_segment_id": "shot-3",
                "left_prompt": "门从旧车站通道打开，视线从门外过渡到地下机房。",
                "right_prompt": "林岚停在地下机房内，回头确认门外动静。",
                "left_scene_binding": {
                    "kind": "transition",
                    "from_scene_profile_id": task["scene_a_id"],
                    "to_scene_profile_id": task["scene_b_id"],
                },
                "right_scene_binding": {"kind": "scene", "scene_profile_id": task["scene_b_id"]},
            },
        )
        if split_second:
            return split_second

        scale = edit("scale", "split_second", {"kind": "scale_timeline", "duration_ms": 12000})
        if scale:
            return scale

        if "final_read" not in observations:
            current = current_resource("scale")
            return host(
                "final_read",
                "read_chapter_visual_plan",
                {
                    **resource,
                    "record_id": int(current["id"]),
                    "expected_revision": int(current["revision"]),
                    "view": {"kind": "outline", "offset": 0, "limit": 20},
                },
            )

        if "validate" not in observations:
            current = current_resource("final_read")
            return host(
                "validate",
                "validate_chapter_visual_plan",
                {
                    **resource,
                    "record_id": int(current["id"]),
                    "expected_revision": int(current["revision"]),
                    "mode": "confirmed",
                },
            )
        if not observations["validate"]["validation"]["valid"]:
            raise ValueError("composed director timeline failed confirmed validation")
        outline = observations["final_read"]["projection"]
        if outline.get("kind") != "outline" or outline.get("total") != 3:
            raise ValueError("composed director timeline did not persist three segments")
        return {"content": "已从 starter 方案完成三段导演时间线，确认前等待用户审阅。"}

    editor = mode in {"editor", "read_editor", "inspect_only"}
    state = None
    if editor:
        if "inspect" not in observations:
            return call("inspect", "inspect_editor_target", {})
        target = observations["inspect"]
        source, state = target["source"], target["state"]
        if mode == "inspect_only":
            return {"content": "已取得原视图的定位与版本。"}
        if "read_editor" not in observations:
            return call("read_editor", "read_editor_draft", {
                "expected_editor_revision": state["editor_revision"],
                "view": {"kind": "segment", "segment_id": source["shot_id"]},
            })
        if mode == "read_editor":
            return {"content": "已读取 editor_draft，未保存。"}
        if state["dirty"] or state["has_uncommitted_dialog"] or task.get("save_editor"):
            if "save_editor" not in observations:
                return call("save_editor", "save_editor_draft", {"expected_editor_revision": state["editor_revision"]})
            if "inspect_saved" not in observations:
                return call("inspect_saved", "inspect_editor_target", {})
            state = observations["inspect_saved"]["state"]
        project_id, chapter_id, shot_id = source["project_id"], source["chapter_id"], source["shot_id"]
    else:
        project_id, chapter_id, shot_id = task["project_id"], task["chapter_id"], task["shot_id"]

    resource = {"project_id": project_id, "source_chapter_id": chapter_id}
    if "shot" not in observations:
        first = host("shot", "read_chapter_visual_plan", {**resource, "view": {"kind": "segment", "segment_id": shot_id}})
        if mode == "batch":
            second = host("source", "read_chapter_visual_plan", {**resource, "view": {"kind": "source", "limit": 30}})
            return {"tool_calls": [first["tool_call"], second["tool_call"]]}
        return first
    saved = observations["shot"]["resource"]
    if "edit" not in observations:
        return host("edit", "edit_chapter_visual_plan_draft", {
            **resource, "record_id": int(saved["id"]), "expected_revision": int(saved["revision"]),
            "expected_source_hash": observations["shot"]["source_hash"],
            "editor_revision": state["editor_revision"] if state else None,
            "operation": {"kind": "update_segment", "segment": {**observations["shot"]["projection"]["segment"],
                          "prompt": task.get("prompt", "门框形成前景遮挡，保留右侧视线空间。")}},
        })
    edited = observations["edit"]
    if "validate" not in observations:
        return host("validate", "validate_chapter_visual_plan", {
            **resource, "record_id": int(edited["resource"]["id"]),
            "expected_revision": int(edited["resource"]["revision"]), "mode": "draft",
        })
    if not observations["validate"]["validation"]["valid"]:
        raise ValueError("saved draft failed validation")
    return {"content": "已保存镜头描述，草稿结构校验通过；等待用户确认。" if edited["changed"] else "字段已相同，未新增修订。"}


def main_fixture(body, messages):
    """Read real child artifacts before invoking the explicit review command."""
    names = {call["id"]: call.get("function", call).get("name")
             for message in messages for call in message.get("tool_calls") or []}
    observations = {names.get(message["tool_call_id"]): json.loads(message["content"])
                    for message in messages if message.get("role") == "tool"}
    for result in observations.values():
        if isinstance(result, dict) and "error" in result:
            return {"content": "任务未完成：" + str(result["error"])}
    visible = {tool["function"]["name"] for tool in body.get("tools", [])}

    def call(name, arguments):
        if body.get("tools") and name not in visible:
            return {"tool_call": {"id": "describe_" + name, "name": "describe_agent_tool", "arguments": {"name": name}}}
        return {"tool_call": {"id": "call_a4n_" + name, "name": name, "arguments": arguments}}

    target = observations.get("locate_writing_target")
    if target is None:
        return call("locate_writing_target", {})
    child = observations.get("delegate")
    if child is None:
        result = call("delegate", {
            "capability": "content_optimization",
            "task": "优化已定位的最新章节正文，生成待用户审阅的局部补丁提案，并提交真实结果。",
            "projectId": int(target["project"]["id"]), "chapterId": int(target["chapter"]["id"]),
            "successCriteria": ["形成可审阅的局部补丁提案", "保持剧情事实与人物设定一致"],
        })
        if result["tool_call"]["name"] == "delegate":
            result["tool_call"]["id"] = "call_a4n_delegate_content_optimization"
        return result
    user_text = "\n".join(message.get("content", "") for message in messages if message.get("role") == "user")
    if "A4N_REVIEW_SKIP" in user_text:
        return {"content": "子任务已返回，尚未验收。"}
    refs = child["result_refs"]
    if "get_generation_task_summary" not in observations:
        task = next(ref for ref in refs if ref["kind"] == "generation_task")
        return call("get_generation_task_summary", {"taskId": task["id"], "attempt": int(task["revision"])})
    session = observations.get("ensure_content_optimization_session")
    if session is None:
        return call("ensure_content_optimization_session", {
            "projectId": int(target["project"]["id"]), "chapterId": int(target["chapter"]["id"]),
        })
    for name in ("list_optimize_turns", "list_optimize_patch_sets"):
        if name not in observations:
            return call(name, {"input": {"session_id": session["id"]}})
    patches = observations["list_optimize_patch_sets"]
    if not any(patch["patches"] and patch["status"] == "pending" for patch in patches):
        raise ValueError("child produced no real pending patch proposal")
    if "review_agent_delegation" not in observations:
        return call("review_agent_delegation", {"input": {
            "delegation_id": child["delegation_id"], "expected_child_run_id": child["child_run_id"],
            "decision": "request_revision" if "A4N_REVIEW_REVISE" in user_text else "accept",
            "reason": "已读取实际补丁、优化答复与任务终态；提案仍待用户确认，未写回正文。",
            "reviewed_refs": refs,
        }})
    return {"content": "已显式审阅正文优化子 Agent 的补丁提案；正文尚未写回。"}
