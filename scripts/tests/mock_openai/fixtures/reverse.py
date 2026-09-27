"""Reverse-planning, agent, embedding, and billing fixtures."""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any

from mock_openai.fixtures.story import blueprint_response

def embedding_for(text: str, dimension: int) -> list[float]:
    digest = hashlib.sha256(text.encode("utf-8")).digest()
    return [round((digest[index % len(digest)] - 127.5) / 127.5, 6) for index in range(dimension)]


def embedded_json_objects(text: str) -> list[dict[str, Any]]:
    decoder = json.JSONDecoder()
    values = []
    for index, character in enumerate(text):
        if character != "{":
            continue
        try:
            value, _ = decoder.raw_decode(text, index)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            values.append(value)
    return values


def reverse_source_context(combined: str) -> dict[str, Any]:
    for value in embedded_json_objects(combined):
        context = value.get("source_context")
        if isinstance(context, dict):
            return context
    return {}


def reverse_blueprint_response() -> dict[str, Any]:
    story = blueprint_response("story_core")
    return {
        **story,
        "world_setting": blueprint_response("world")["world_setting"],
        "characters": blueprint_response("characters")["characters"],
        "relationships": blueprint_response("relationships")["relationships"],
    }


def reverse_outline_response(combined: str) -> dict[str, Any]:
    summaries = reverse_source_context(combined).get("summaries")
    if not isinstance(summaries, list) or not summaries:
        summaries = [{"chapter_number": 1, "title": "雨夜来信"}]
    outlines = []
    for index, item in enumerate(summaries, 1):
        item = item if isinstance(item, dict) else {}
        chapter_number = item.get("chapter_number", index)
        title = item.get("title") or f"第{chapter_number}章"
        outlines.append(
            {
                "chapter_number": chapter_number,
                "title": title,
                "summary": f"第{chapter_number}章围绕{title}推进：林岚核验时间异常线索，与周衡共同追查旧车站钟楼，并在新的证据中逼近未来来信的真相。",
            }
        )
    return {"outlines": outlines}


def reverse_part_outline_response(combined: str) -> dict[str, Any]:
    context = reverse_source_context(combined)
    outlines = context.get("chapter_outlines")
    chapter_numbers = [
        item.get("chapter_number")
        for item in outlines or []
        if isinstance(item, dict) and isinstance(item.get("chapter_number"), int)
    ]
    total = context.get("total_chapters")
    if not isinstance(total, int) or total <= 0:
        total = max(chapter_numbers, default=1)
    parts = []
    for part_number, start in enumerate(range(1, total + 1, 25), 1):
        end = min(start + 24, total)
        parts.append(
            {
                "part_number": part_number,
                "title": f"第{part_number}部·失去的时间",
                "start_chapter": start,
                "end_chapter": end,
                "summary": "林岚沿未来来信追查旧车站的七分钟偏差，在与周衡互相试探、共同取证的过程中逐步确认实验钟正在改写城市记忆，并逼近顾沉试图挽回过去的真实目的。",
                "theme": "以求证建立信任并接受真实的失去",
                "key_events": ["未来来信预告停电", "地下机房发生时间回卷", "钟楼实验的代价被揭示"],
                "character_focus": {"林岚": "从独自承担秘密转向信任同伴，并选择守护真实时间线。"},
            }
        )
    return {"parts": parts}


def agent_chat_fixture(body: dict[str, Any]) -> dict[str, Any] | None:
    messages = body.get("messages")
    if not isinstance(messages, list):
        return None
    messages = [dict(message) for message in messages]
    for message in messages:
        content = message.get("content")
        if message.get("role") == "user" and isinstance(content, str) and content.startswith("[可变上下文]"):
            message["content"] = content.split("\n[当前任务]\n", 1)[1]
    if any(message.get("role") == "system" and str(message.get("content", "")).startswith("你负责压缩 A4N Agent 的历史") for message in messages):
        source = json.loads(next(message["content"] for message in reversed(messages) if message.get("role") == "user"))
        from specialized_agent_fixture import summary_fixture
        return summary_fixture(source)

    # The main Agent thread is durable, so a new turn carries the previous
    # turns' tool calls and child results as well.  Fixture decisions must be
    # scoped to the latest real user message; otherwise an old delegation can
    # make a fresh request look already completed. Runtime state is appended
    # after tool observations and is not a new user turn.
    current_turn_start = next(
        (
            index
            for index in range(len(messages) - 1, -1, -1)
            if isinstance(messages[index], dict)
            and messages[index].get("role") == "user"
            and not str(messages[index].get("content", "")).startswith(
                ("[Agent inbox:", "当前执行阶段：")
            )
        ),
        0,
    )
    current_messages = messages[current_turn_start:]
    if any(str(message.get("content", "")).startswith("A4N_RELIABILITY ") for message in messages if message.get("role") == "user"):
        from specialized_agent_fixture import reliability_fixture
        return reliability_fixture(body, messages)

    capability = None
    for message in messages:
        if not isinstance(message, dict) or message.get("role") != "system":
            continue
        content = message.get("content")
        if not isinstance(content, str) or "你是 A4N 项目 Agent。" not in content:
            continue
        match = re.search(r"当前能力域(?:为|[:：])\s*([a-z0-9_]+)", content)
        if match:
            capability = match.group(1)
            break
    if capability is None:
        return None

    if any(
        str(message.get("content", "")).startswith("A4N_GENERAL ")
        for message in current_messages
        if message.get("role") == "user"
    ):
        from specialized_agent_fixture import general_fixture
        return general_fixture(body, messages)
    if capability in {"director", "canvas"} or any(
        str(message.get("content", "")).startswith("A4N_SPECIALIZED ")
        for message in current_messages if message.get("role") == "user"
    ):
        from specialized_agent_fixture import director_fixture
        return director_fixture(body, messages)
    if capability == "content_optimization":
        from specialized_agent_fixture import optimization_fixture
        return optimization_fixture(body, current_messages)
    if capability != "main":
        return None
    from specialized_agent_fixture import main_fixture
    return main_fixture(body, current_messages)


def message_text(content: Any) -> str:
    if isinstance(content, str):
        return content
    if not isinstance(content, list) or any(
        not isinstance(part, dict) or not isinstance(part.get("text"), str)
        for part in content
    ):
        raise ValueError("text fixture requires text content blocks")
    return "".join(part["text"] for part in content)


def cache_fixture_usage(protocol: str) -> dict[str, Any]:
    """Fixed billing counters test normalization, not a simulated cache engine."""
    if protocol == "anthropic":
        return {"input_tokens": 600, "cache_read_input_tokens": 9000, "cache_creation_input_tokens": 400, "output_tokens": 0}
    if protocol == "gemini":
        return {"promptTokenCount": 10000, "cachedContentTokenCount": 9000, "candidatesTokenCount": 100, "totalTokenCount": 10100}
    if protocol == "responses":
        return {"input_tokens": 10000, "output_tokens": 100, "total_tokens": 10100,
                "input_tokens_details": {"cached_tokens": 9000, "cache_write_tokens": 400}}
    return {"prompt_tokens": 10000, "completion_tokens": 100, "total_tokens": 10100,
            "prompt_tokens_details": {"cached_tokens": 9000, "cache_write_tokens": 400}}
