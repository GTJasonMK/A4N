fn sample_analysis() -> Value {
    serde_json::json!({
        "metadata": {
            "characters": ["林岚"],
            "locations": ["旧城"],
            "items": [],
            "tags": ["转折"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "林岚在旧城发现新的线索，并意识到此前事件并非偶然。",
            "one_line": "林岚发现旧城线索。",
            "keywords": ["旧城", "线索", "转折"]
        },
        "character_states": {
            "林岚": {
                "location": "旧城",
                "status": "追查线索",
                "changes": ["发现隐藏标记"],
                "emotional_state": "警觉"
            }
        },
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": []
        },
        "key_events": [
            {
                "type": "discovery",
                "description": "发现隐藏标记",
                "importance": "high",
                "involved_characters": ["林岚"]
            }
        ]
    })
}
