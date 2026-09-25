"""Deterministic expository chapters for cadence and recovery tests."""
import re
from .reverse import embedded_json_objects
from .entities import chapter_analysis_foreshadowing_response


def identity_feedback_response(combined: str) -> dict:
    result = chapter_analysis_foreshadowing_response(combined)
    existing_ids = {item["id"] for item in embedded_json_objects(combined)
                    if isinstance(item.get("id"), str) and item.get("setup_chapter") == 1
                    and "影子" in item.get("description", "")}
    if len(existing_ids) > 1:
        raise ValueError("identity fixture expects one existing foreshadowing")
    if existing_ids:
        result["foreshadowing"]["planted"][0].update(
            id=existing_ids.pop(), description="周衡影子的异常暗示他与时间裂缝有关。")
    return result


def fact_feedback_response(combined: str, invalid: bool = False) -> dict:
    objects = embedded_json_objects(combined)
    spans = [item for item in objects if isinstance(item.get("id"), str)
             and re.fullmatch(r"ch\d+:s\d+:f\d+", item["id"]) and isinstance(item.get("text"), str)]
    if "project_fact_consolidation" in combined:
        materials = next(item for item in objects if "blueprint" in item and "part_outlines" in item)
        blueprint = materials["blueprint"]
        if not blueprint:
            return {"changes": []}
        current = blueprint["world_setting"].get("culture_and_society", {}).get("calendar")
        applicable = [span for span in spans if "每年有" in span["text"]]
        latest = max(applicable, key=lambda span: int(re.match(r"ch(\d+)", span["id"])[1]))
        value = calendar_fact(latest["text"])
        if current == value:
            return {"changes": []}
        return {"changes": [{
            "material_kind": "world_setting", "target_id": blueprint["id"],
            "path": "/culture_and_society/calendar", "operation": "replace",
            "before": current, "after": value, "reason": "正文明确说明记忆城的历法",
            "source_span_ids": ["ch999:s1:f1" if invalid else latest["id"]],
        }]}
    if "chapter_fact_extraction" in combined:
        span = next(span for span in spans if "每年有" in span["text"])
        return {"characters": [], "locations": [], "items": [], "event_candidates": [],
                "foreshadowing_candidates": [], "world_facts": [{
                    "field": "culture_and_society", "subject": "记忆城历法", "statement": calendar_fact(span["text"]),
                    "assertion": "observed", "source_span_ids": [span["id"]],
                }]}
    if "chapter_summary_synthesis" in combined:
        return {"metadata_additions": {"tags": ["世界设定"], "tone": "平静", "timeline_marker": None},
                "summaries": {"compressed": "本章说明记忆城的历法。", "one_line": "介绍记忆城历法。", "keywords": ["历法"]}}
    if "chapter_character_state" in combined:
        return {"character_states": {}}
    if "chapter_foreshadowing_events" in combined:
        return {"foreshadowing": {"planted": [], "resolved": [], "progressed": [], "tensions": []}, "key_events": []}
    for name, field in [("object_discovery", "objects"), ("character_discovery", "characters"),
                        ("scene_profile_candidates", "scenes"), ("scene_occurrence_extraction", "occurrences"),
                        ("scene_transition_extraction", "transitions"), ("scene_entity_state_extraction", "entity_states")]:
        if f'"name": "{name}"' in combined or f'"name":"{name}"' in combined:
            return {field: []}
    raise ValueError("unexpected prompt in fact feedback fixture")


def calendar_fact(text: str) -> str:
    if "每年有七个季节" in text:
        return "记忆城每年有七个季节"
    if "每年有六个季节" in text:
        return "记忆城每年有六个季节"
    raise ValueError("fixture chapter must explicitly describe its calendar")
