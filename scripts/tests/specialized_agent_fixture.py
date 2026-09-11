"""Deterministic Director decisions; IDs and versions come from real observations."""

import json


MARKER = "A4N_SPECIALIZED "


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
                "view": {"kind": "shot", "shot_id": source["shot_id"]},
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
        first = host("shot", "read_chapter_visual_plan", {**resource, "view": {"kind": "shot", "shot_id": shot_id}})
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
            "operation": {"kind": "update_shot_fields", "shot_id": shot_id,
                          "changes": {"composition": task.get("composition", "门框形成前景遮挡，保留右侧视线空间。")}},
        })
    edited = observations["edit"]
    if "validate" not in observations:
        return host("validate", "validate_chapter_visual_plan", {
            **resource, "record_id": int(edited["resource"]["id"]),
            "expected_revision": int(edited["resource"]["revision"]), "mode": "draft",
        })
    if not observations["validate"]["validation"]["valid"]:
        raise ValueError("saved draft failed validation")
    return {"content": "已保存构图说明，草稿结构校验通过；等待用户确认。" if edited["changed"] else "字段已相同，未新增修订。"}


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
