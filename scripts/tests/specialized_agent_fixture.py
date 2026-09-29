"""Deterministic Director decisions; IDs and versions come from real observations."""

import hashlib
import json


MARKER = "A4N_SPECIALIZED "
GENERAL_MARKER = "A4N_GENERAL "
STEP_MARKER = "A4N_FIXTURE_CALLS "
SUMMARY_STATE_MARKER = "A4N_FIXTURE_STATE "


def fixture_call(key, name, arguments):
    # A provider may rename/scoped call IDs before replay. Fixture state belongs
    # to the assistant decision, not to the transport's opaque identifiers.
    return {"content": STEP_MARKER + json.dumps([key]),
            "tool_call": {"id": key, "name": name, "arguments": arguments}}


def logical_call_ids(messages):
    result = {}
    for message in messages:
        calls = message.get("tool_calls") or []
        content = message.get("content") or ""
        keys = json.loads(content[len(STEP_MARKER):]) if content.startswith(STEP_MARKER) else [call["id"] for call in calls]
        if len(keys) != len(calls):
            raise ValueError("fixture call labels do not match its tool batch")
        result.update((call["id"], key) for call, key in zip(calls, keys))
    return result


def summary_fixture(source):
    previous = source.get("previous_summary") or {}
    current = source.get("current_request") or []
    input_id = current[0]["message_id"] if current else None
    constraints = list(previous.get("constraints", []))
    for record in source["messages"]:
        message = record["message"]
        if message["role"] == "user" and "约束" in message["content"] and message["content"] not in constraints:
            constraints.append(message["content"])
    states = {}
    for item in previous.get("tool_state", []):
        if item.startswith(SUMMARY_STATE_MARKER):
            state = json.loads(item[len(SUMMARY_STATE_MARKER):])
            if state["input_id"] == input_id:
                states[state["key"]] = state
    records = [record for record in source["messages"] if input_id is not None and record["message_id"] >= input_id]
    aliases = logical_call_ids([record["message"] for record in records])
    names = {call["id"]: call.get("function", call)["name"] for record in records for call in record["message"].get("tool_calls") or []}
    for record in records:
        message = record["message"]
        if message["role"] != "tool":
            continue
        call_id = message["tool_call_id"]
        name = names[call_id]
        value = json.loads(message["content"]) if name == "delegate" else observation_data(message)
        states[aliases[call_id]] = {"input_id": input_id, "key": aliases[call_id], "name": name, "data": value}
    tool_state = [SUMMARY_STATE_MARKER + json.dumps(state, ensure_ascii=False) for state in states.values()]
    if len(tool_state) > 64 or any(len(item) > 4000 for item in tool_state):
        raise ValueError("fixture tool state exceeds the summary contract")
    goal = current[-1]["content"] if current else previous.get("current_goal", "继续当前任务")
    return {"content": json.dumps({
        "schema_version": 2, "current_goal": goal, "constraints": constraints,
        "decisions": previous.get("decisions", []), "facts_resources": previous.get("facts_resources", []),
        "completed": ["工具调用已返回：" + state["name"] for state in states.values() if not (isinstance(state["data"], dict) and "error" in state["data"])],
        "pending": [goal], "tool_state": tool_state,
        "covered_through_sequence": source["covered_through_sequence"],
    }, ensure_ascii=False)}


def summarized_observations(messages, start):
    # Compacted current-turn input is pinned before the retained tool batch.
    # A fresh turn still follows that older batch and must not inherit its steps.
    first = next((index for index, message in enumerate(messages) if message["role"] != "system"), None)
    if first != start:
        return {}
    summaries = [message["content"] for message in messages if message["role"] == "system" and "<context_summary>" in str(message.get("content", ""))]
    if not summaries:
        return {}
    summary = json.loads(summaries[-1].split("<context_summary>", 1)[1].split("</context_summary>", 1)[0])
    if summary["current_goal"] != messages[start]["content"]:
        return {}
    return {state["key"]: state["data"] for item in summary["tool_state"] if item.startswith(SUMMARY_STATE_MARKER)
            for state in [json.loads(item[len(SUMMARY_STATE_MARKER):])]}


def reliability_fixture(body, messages):
    marker = "A4N_RELIABILITY "
    starts = [index for index, message in enumerate(messages) if message.get("role") == "user" and str(message.get("content", "")).startswith(marker)]
    if not starts:
        return None
    start = starts[-1]
    task = json.JSONDecoder().raw_decode(messages[start]["content"][len(marker):])[0]
    current = messages[start:]
    aliases = logical_call_ids(current)
    observations = {}
    for message in current:
        if message.get("role") == "tool":
            observations[aliases[message["tool_call_id"]]] = observation_data(message)
    mode = task["mode"]
    if mode == "summary_final":
        summaries = [message["content"] for message in messages if message.get("role") == "system" and "<context_summary>" in str(message.get("content", ""))]
        if not summaries or "白塔" not in summaries[-1]:
            raise ValueError("compaction lost the original constraint")
        return {"content": "白塔"}
    if mode == "question":
        if "ask" not in observations:
            return fixture_call("ask", "request_agent_input", {"question": "请输入验收数字。"})
        if not any(message.get("role") == "user" and message.get("content") == "42" for message in current):
            raise ValueError("question resumed without its user answer")
        return {"content": "已收到42"}
    if mode == "unknown_tool":
        if "bad" not in observations:
            result = fixture_call("bad", "a4n_tool_that_does_not_exist", {})
            result["tool_call"]["id"] = "repeated-provider-id"
            return result
        if "error" not in observations["bad"]:
            raise ValueError("unknown tool was not reported as an error")
        if "project" not in observations:
            result = fixture_call("project", "get_project", {"projectId": task["project_id"]})
            result["tool_call"]["id"] = "repeated-provider-id"
            return result
        return {"content": observations["project"]["title"]}
    if mode == "plain":
        return {"content": "验收完成"}
    raise ValueError("unknown reliability fixture mode")


def observation_data(message):
    observation = json.loads(message["content"])
    if observation.get("observation_version") != 1:
        raise ValueError("Agent fixture requires observation_version 1")
    if observation["status"] == "error":
        return {"error": observation["error"]}
    if observation["status"] not in {"success", "partial"}:
        raise ValueError("Agent fixture received an unknown observation status")
    return observation["data"]


def optimization_fixture(body, messages):
    user_text = "\n".join(message.get("content", "") for message in messages if message.get("role") == "user")
    if "A4N_OPTIMIZATION_PLAIN" in user_text:
        return {"content": "补丁须经用户确认才写回正文。"}
    names = {call["id"]: call.get("function", call)["name"]
             for message in messages for call in message.get("tool_calls") or []}
    observations = {names[message["tool_call_id"]]: observation_data(message)
                    for message in messages if message.get("role") == "tool"}
    for result in observations.values():
        if isinstance(result, dict) and "error" in result:
            raise ValueError("optimization fixture tool failed: " + str(result["error"]))
    if "A4N_OPTIMIZATION_CONTEXT" in user_text:
        aliases = logical_call_ids(messages)
        results = {aliases[message["tool_call_id"]]: observation_data(message)
                   for message in messages if message.get("role") == "tool"}
        if "find" not in results:
            return fixture_call("find", "search_project_context", {"source": "current_chapter", "query": "第二段"})
        if "read_match" not in results:
            hit = results["find"]["matches"][0]
            return fixture_call("read_match", "read_project_context", {
                "source": hit["source"], "expected_hash": hit["resource"]["hash"],
                "selection": {"kind": "paths", "paths": [hit["path"]]}, "include": ["content"],
            })
        if "第二段" not in results["read_match"]["content"][0]["value"]:
            raise ValueError("search result did not read back its matching source")
        if "read_second" not in results:
            return fixture_call("read_second", "read_project_context", {
                "source": "current_chapter", "selection": {"kind": "paragraphs", "indexes": [2]},
                "include": ["content", "source_spans"],
            })
        paragraph = results["read_second"]["content"]["paragraphs"][0]
        if paragraph["content"] != "第二段。" or paragraph["source_spans"][0]["text"] != "第二段":
            raise ValueError("paragraph text and evidence must come from the same source range")
        return {"content": "搜索路径可以读取，第二段与引用一致。"}
    if "inspect_project_context" not in observations:
        return fixture_call("inspect_context", "inspect_project_context", {"source": "current_chapter"})
    if "read_project_context" not in observations:
        return {"tool_call": {"id": "call_a4n_read_optimization", "name": "read_project_context", "arguments": {
            "source": "current_chapter", "selection": {"kind": "paragraphs", "indexes": [1]},
            "include": ["content", "source_spans"],
        }}}
    if "edit_article" not in observations:
        source = observations["read_project_context"]
        if source.get("read_tool") == "read_agent_execution_result":
            if "read_agent_execution_result" not in observations:
                return fixture_call("read_original", "read_agent_execution_result", {
                    "execution_id": source["execution_id"], "pointer": "/content/paragraphs/0/content", "offset": 0, "limit": 2000,
                })
            original = observations["read_agent_execution_result"]["text"]
        else:
            original = source["content"]["paragraphs"][0]["content"]
        if not original:
            raise ValueError("optimization fixture requires a real chapter paragraph")
        original = original[:original.index("。") + 1]
        ending = "她将信封压在掌心，没有立即拆开。" if "A4N_REVISION_SECOND" in user_text else "她握紧信封，停下脚步。"
        return {"tool_call": {"id": "call_a4n_propose_optimization", "name": "edit_article", "arguments": {
            "summary": "局部表达优化提案", "patches": [{
                "patch_id": "patch-local-1", "issue_id": None, "operation": "replace",
                "old_text": original, "new_text": original + ending,
                "reason": "以可见动作承接原文。", "risk": "low",
            }],
        }}}
    proposal = observations["edit_article"]
    if proposal.get("mode") != "pending_confirmation" or not proposal.get("patch_set_id"):
        raise ValueError("optimization fixture requires a persisted pending proposal")
    return {"content": "已生成局部补丁提案，等待用户审阅；交付说明：正文尚未写回。"}


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
    logical_ids = logical_call_ids(messages[start:])
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
            observations[logical_ids[call_id]] = observation_data(message)
    if pending:
        raise ValueError("model was called before its tool batch was resolved")
    if any("error" in result for result in observations.values()):
        return {"content": "通用 Agent 操作未完成。"}

    def call(key, name, arguments):
        return fixture_call(key, name, arguments)

    def host(key, name, arguments):
        return call(key, name, {"input": arguments})

    if "search" not in observations:
        return call("search", "search_agent_tools", {"query": "get_project"})
    if "describe" not in observations:
        return call("describe", "describe_agent_tool", {"name": "get_project"})
    if "project" not in observations:
        return call("project", "get_project", {"projectId": task["project_id"]})
    return {"content": "已通过工具发现读取项目，并取得真实项目结果。"}


def director_fixture(body, messages):
    start = next((i for i in range(len(messages) - 1, -1, -1)
                  if messages[i].get("role") == "user"
                  and str(messages[i].get("content", "")).startswith(MARKER)), None)
    if start is None:
        return None
    task = json.loads(messages[start]["content"][len(MARKER):])
    logical_ids = logical_call_ids(messages[start:])
    observations = summarized_observations(messages, start)
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
            observations[logical_ids[message["tool_call_id"]]] = observation_data(message)
    for result in observations.values():
        if "error" in result:
            return {"content": "操作未完成：" + str(result["error"])}

    def call(key, name, arguments):
        return fixture_call(key, name, arguments)

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
            return call(
                "run",
                "run_canvas_node",
                {"input": {**resource, "node_id": task["node_id"]}},
            )
        if task.get("run") and "wait" not in observations:
            return host(
                "wait",
                "wait_workflow_run",
                {
                    "workflow_run_id": observations["run"]["workflow_run_id"],
                    "include_generation_runs": False,
                },
            )
        return {"content": "画布局部修改已保存，并已取得实际执行结果。" if task.get("run") else "画布局部修改已保存。"}
    if mode == "read_result":
        if "result" not in observations:
            return call("result", "read_agent_execution_result", {
                "execution_id": task["execution_id"], "pointer": "/source/content", "offset": 2, "limit": 8,
            })
        return {"content": "已读取历史工具结果：" + observations["result"]["text"]}
    if mode == "discover":
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
                {
                    "project_id": task["project_id"],
                    "generation_run_id": int(result_ref["id"]),
                },
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
            return host(
                "preview",
                "preview_visual_plan_point_generation",
                {"project_id": task["project_id"], "options": options},
            )
        preview = observations["preview"]
        if preview.get("complete") is False:
            execution_id = preview["execution_id"]
            if "preview_aspect" not in observations:
                return call(
                    "preview_aspect",
                    "read_agent_execution_result",
                    {"execution_id": execution_id, "pointer": "/aspect_ratio"},
                )
            if "preview_refs" not in observations:
                return call(
                    "preview_refs",
                    "read_agent_execution_result",
                    {"execution_id": execution_id, "pointer": "/reference_image_ids"},
                )
            preview = {
                "aspect_ratio": observations["preview_aspect"]["text"],
                "reference_image_ids": json.loads(observations["preview_refs"]["text"]),
            }
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
        images = observations["images"]
        if isinstance(images, dict) and images.get("complete") is False:
            if "image_metadata_0" not in observations:
                return call(
                    "image_metadata_0",
                    "read_agent_execution_result",
                    {"execution_id": images["execution_id"], "pointer": "/0/metadata_json/director_candidate"},
                )
            first_metadata = json.loads(observations["image_metadata_0"]["text"])
            if first_metadata.get("point_id") != task["point_id"]:
                if "image_metadata_1" not in observations:
                    return call(
                        "image_metadata_1",
                        "read_agent_execution_result",
                        {"execution_id": images["execution_id"], "pointer": "/1/metadata_json/director_candidate"},
                    )
                images = [
                    {"metadata_json": {"director_candidate": first_metadata}},
                    {
                        "metadata_json": {
                            "director_candidate": json.loads(observations["image_metadata_1"]["text"])
                        }
                    },
                ]
            else:
                images = [{"metadata_json": {"director_candidate": first_metadata}}]
        image = next(
            (
                item
                for item in images
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

    editor = mode in {"editor", "read_editor", "inspect_only", "view_controls"}
    state = None
    if editor:
        if "inspect" not in observations:
            return call("inspect", "inspect_editor_target", {})
        target = observations["inspect"]
        source, state = target["source"], target["state"]
        if mode == "view_controls":
            for index, action in enumerate(task["actions"]):
                key = f"control_{index}"
                if key not in observations:
                    return call(key, "control_editor_view", {"action": action})
            return {"content": "已按明确请求调整原时间线视图，未修改保存方案。"}
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
            return {"content": STEP_MARKER + json.dumps(["shot", "source"]), "tool_calls": [first["tool_call"], second["tool_call"]]}
        return first
    saved = observations["shot"]["resource"]
    if "edit" not in observations:
        operation = {"kind": "update_segment", "segment": {**observations["shot"]["projection"]["segment"],
                     "prompt": task.get("prompt", "门框形成前景遮挡，保留右侧视线空间。")}}
        if "batch_operations" in task:
            operation = {"kind": "batch", "operations": [operation, *task["batch_operations"]]}
        return host("edit", "edit_chapter_visual_plan_draft", {
            **resource, "record_id": int(saved["id"]), "expected_revision": int(saved["revision"]),
            "expected_source_hash": observations["shot"]["source_hash"],
            "editor_revision": state["editor_revision"] if state else None,
            "operation": operation,
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
    logical_ids = logical_call_ids(messages)
    names = {logical_ids[call["id"]]: call.get("function", call).get("name")
             for message in messages for call in message.get("tool_calls") or []}
    results = {}
    for message in messages:
        if message.get("role") != "tool":
            continue
        call_id = logical_ids[message["tool_call_id"]]
        if names.get(call_id) == "delegate":
            # Durable child completion uses the delegation result contract,
            # independently of ordinary Host tool observations.
            result = json.loads(message["content"])
            if not all(isinstance(result.get(key), str) and result[key]
                       for key in ("delegation_id", "child_run_id")):
                raise ValueError("Agent fixture requires a durable delegation result")
            results[call_id] = result
        else:
            results[call_id] = observation_data(message)
    observations = {names.get(call_id): result for call_id, result in results.items()}
    for result in observations.values():
        if isinstance(result, dict) and "error" in result:
            return {"content": "任务未完成：" + str(result["error"])}
    visible = {tool["function"]["name"] for tool in body.get("tools", [])}

    def call(name, arguments, call_id=None):
        if body.get("tools") and name not in visible:
            return fixture_call("describe_" + name, "describe_agent_tool", {"name": name})
        return fixture_call(call_id or "call_a4n_" + name, name, arguments)

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
    revised = False
    for message in messages:
        content = message.get("content") or ""
        if message.get("role") == "assistant" and content.startswith("{"):
            inbox = json.loads(content)
            if inbox.get("source_kind") == "subagent" and inbox.get("inbox_kind") == "child_result":
                payload = inbox["payload"]
                if payload["delegation_id"] == child["delegation_id"]:
                    child = payload
                    revised = True
    user_text = "\n".join(message.get("content", "") for message in messages if message.get("role") == "user")
    if "A4N_REVIEW_SKIP" in user_text:
        return {"content": "子任务已返回，尚未验收。"}
    refs = child["result_refs"]
    readers = {"optimization_patch_set": "list_optimize_patch_sets", "optimization_turn": "list_optimize_turns"}
    if not refs or any(ref["kind"] not in readers for ref in refs):
        raise ValueError("child must deliver versioned optimization artifacts")
    sessions_key = "sessions_" + child["child_run_id"]
    sessions = results.get(sessions_key)
    if sessions is None:
        return call("list_optimize_sessions", {"input": {
            "project_id": int(target["project"]["id"]), "chapter_id": int(target["chapter"]["id"]),
        }}, sessions_key)
    observed = []
    for session in sessions:
        for kind, reader in readers.items():
            if not any(ref["kind"] == kind for ref in refs):
                continue
            call_id = f"call_a4n_{reader}_{session['id']}_{child['child_run_id']}"
            if call_id not in results:
                return call(reader, {"input": {"session_id": session["id"]}}, call_id)
            for row in results[call_id]:
                if kind == "optimization_patch_set":
                    if not row["patches"] or row["status"] != "pending":
                        continue
                    revision, digest = row["updated_at"], row["base_content_hash"]
                else:
                    if row["final_reply"] is None:
                        continue
                    revision, digest = hashlib.sha256(row["final_reply"].encode()).hexdigest(), None
                observed.append({"kind": kind, "id": str(row["id"]), "revision": revision, "hash": digest})
    if any(not any(all(ref.get(key) == row.get(key) for key in ("kind", "id", "revision", "hash"))
                   for row in observed) for ref in refs):
        raise ValueError("child artifacts are missing, not pending, or have changed versions")
    review_key = "review_" + child["child_run_id"]
    if review_key not in results:
        request_revision = "A4N_REVIEW_REVISE" in user_text and not revised
        return call("review_agent_delegation", {"input": {
            "delegation_id": child["delegation_id"], "expected_child_run_id": child["child_run_id"],
            "decision": "request_revision" if request_revision else "accept",
            "reason": "A4N_REVISION_SECOND 请把新增动作改为将信封压在掌心、没有立即拆开。" if request_revision else "已核对交付的优化资源及版本；提案仍待用户确认，未写回正文。",
            "reviewed_refs": refs,
        }}, review_key)
    return {"content": "已显式审阅正文优化子 Agent 的补丁提案；正文尚未写回。"}
