use serde_json::{Value, json};

fn world_setting() -> Value {
    json!({
        "premise": "旧城被星潮改写记忆。",
        "core_rules": "记忆会被周期性校验和封存。",
        "technology_or_magic": {
            "source": "星潮辐射",
            "rules": ["记忆可比对"],
            "limits": ["被污染记忆不可直接读取"],
            "costs": ["审校会损耗自我认知"]
        },
        "key_locations": [
            {"name": "旧城档案馆", "description": "保存纸档", "story_function": "调查据点"}
        ],
        "factions": [
            {"name": "调查局", "description": "管理记忆秩序", "agenda": "封锁真相", "representative_characters": ["林澈"]}
        ],
        "history_timeline": [
            {"period": "灾变元年", "event": "第一场星潮", "impact": "审校制度建立"}
        ],
        "culture_and_society": {
            "values": "稳定高于真实",
            "class_structure": "审校员掌握信息优势",
            "daily_life": "居民定期接受校验"
        },
        "conflict_sources": [
            {"name": "真相封锁", "description": "调查局隐藏源头", "stakes": "居民继续失去自我"}
        ],
        "mysteries_and_foreshadowing": [
            {"mystery": "星潮源头", "early_clues": ["重复档案"], "payoff_direction": "源头与主角家族有关"}
        ],
        "genre_promises": ["记忆悬疑"]
    })
}

fn character(name: &str, character_type: &str) -> Value {
    json!({
        "name": name,
        "character_type": character_type,
        "gender": "未知",
        "age": "未知",
        "appearance": "有明确视觉识别点",
        "personality": "谨慎、执拗",
        "background": "旧城居民",
        "motivation": "查明真相",
        "arc_summary": "从旁观者变为行动者",
        "relationship_to_protagonist": "本人或重要关系人",
        "arc_stages": {
            "start": "相信秩序",
            "core_wound": "失去重要记忆",
            "desire": "找回真相",
            "need": "面对代价",
            "secret": "未知",
            "voice_style": "克制直接",
            "turning_point": "发现档案被篡改",
            "end": "公开真相"
        }
    })
}

pub(super) fn canonical_payload() -> Value {
    json!({
        "world_setting": world_setting(),
        "characters": [
            character("林澈", "protagonist"),
            character("沈砚", "antagonist"),
            character("许南星", "supporting")
        ],
        "relationships": [{
            "relationship_type": "rival",
            "character_a": "林澈",
            "character_b": "沈砚",
            "description": "一方追查真相，一方维护封锁。",
            "strength": 8,
            "evolution": {
                "start": "上下级信任",
                "tension": "调查方向被阻断",
                "turning_points": ["档案馆被封"],
                "hidden_truth": "沈砚知道失踪真相",
                "story_function": "推动制度冲突",
                "end": "彻底对立"
            }
        }]
    })
}
