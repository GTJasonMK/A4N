#!/usr/bin/env python3
"""Deterministic loopback-only OpenAI-compatible provider for CLI E2E tests.

The implementation lives in named fixture and transport modules. This file
keeps the historical import surface and executable path stable.
"""

from __future__ import annotations

from mock_openai_fixture_adaptations import (
    galgame_branch_design_response,
    galgame_character_staging_response,
    galgame_script_adaptation_response,
    galgame_shared_step,
    galgame_source_span_id,
    galgame_staged_line,
    galgame_visible_cast,
    galgame_visual_planning_response,
    manga_available_source_span_ids,
    manga_beat_ref,
    manga_character_binding_response,
    manga_drawing_prompt_response,
    manga_item_response,
    manga_page_plan_response,
    manga_prompt_optimization_response,
    manga_speech_selection_response,
    manga_synthesis_response,
    manga_visual_anchor_response,
)
from mock_openai_fixture_entities import (
    character_discovery_response,
    character_sync_attribute_changes_response,
    character_sync_behaviors_response,
    character_sync_chapter_states_response,
    character_sync_evidence_events_response,
    character_sync_profile_name,
    chapter_analysis_character_state_response,
    chapter_analysis_facts_response,
    chapter_analysis_foreshadowing_response,
    chapter_analysis_summary_response,
    implicit_attribute_classification_response,
    named_profile_id,
    object_discovery_response,
    optimization_analysis_response,
    optimization_chat_response,
    optimization_patch_response,
    scene_entity_state_response,
    scene_occurrence_response,
    scene_profile_candidates_response,
    scene_transition_response,
    source_span_id,
    source_span_pairs,
    text_anchor_id,
)
from mock_openai_fixture_media import chapter_visual_plan_fixture
from mock_openai_fixture_reverse import (
    agent_chat_fixture,
    cache_fixture_usage,
    embedding_for,
    embedded_json_objects,
    message_text,
    reverse_blueprint_response,
    reverse_outline_response,
    reverse_part_outline_response,
)
from mock_openai_fixture_shooting import (
    chat_video_prompt_response,
    shooting_adaptation_response,
    shooting_design_input,
    shooting_design_response,
    shooting_design_scene_template,
    shooting_layout_response,
    shooting_span_ids,
    video_prompt_response,
    video_segment_plan_response,
    video_static_frame_response,
    video_storyboard_frame_response,
)
from mock_openai_fixture_story import (
    blueprint_response,
    chapter_body_response,
    chapter_outline_response,
    character,
    inspiration_response,
    part_outline_response,
)
from mock_openai_prompt_parsing import current_task_prompt_name, json_task_payload
from mock_openai_provider_protocol import (
    MP4_1S_BASE64,
    PNG_1X1_BASE64,
    UnhandledPromptError,
    VideoIdempotencyConflict,
)
from mock_openai_provider_response import response_for
from mock_openai_provider_transport import ProviderHandler, ProviderServer, main


if __name__ == "__main__":
    main()
