"""Binary and image-plan fixtures used by workflow tests."""

from __future__ import annotations

from typing import Any
def chapter_visual_plan_fixture(scene_profile_id: int) -> dict[str, Any]:
    return {
        "schema_version": "chapter_visual_plan_v3",
        "frame": {"width": 1024, "height": 1024},
        "points": [
            {"id": point_id, "at_ms": at_ms, "label": point_id,
             "image_id": None, "description": "车站中的分镜画面", "reference_image_ids": []}
            for point_id, at_ms in [("start", 0), ("end", 1000)]
        ],
        "segments": [{"id": "shot_fixture", "title": "Fixture shot", "start_point_id": "start", "end_point_id": "end",
                      "prompt": "林岚停在通道入口，抬头观察墙上的钟。", "scene_binding": {"kind": "scene", "scene_profile_id": scene_profile_id},
                      "use_end_point_image": False}],
    }
