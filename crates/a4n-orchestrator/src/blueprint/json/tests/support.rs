use super::*;
use a4n_core::error::AppError;
use serde_json::json;

fn valid_blueprint() -> String {
    json!({
        "title": "星潮档案",
        "target_audience": "成年幻想读者",
        "genre": "科幻奇幻",
        "style": "冷峻写实",
        "tone": "悬疑",
        "one_sentence_summary": "档案员追查星潮真相并重建失落记忆。",
        "full_synopsis": "主角在星潮灾变后进入档案城，逐步发现记忆交易背后的统治机制，并在同伴与敌手的夹击中选择公开真相。",
        "world_setting": {
            "premise": "星潮灾变后，记忆成为可被封存、审校和买卖的公共资源。",
            "core_rules": "记忆可被封存和交易。",
            "technology_or_magic": {
                "source": "星潮残留物与档案术共同塑造记忆存储技术",
                "rules": ["记忆封存必须绑定原主情绪", "公开记忆会改变城市信用"],
                "limits": ["被污染的记忆无法直接读取"],
                "costs": ["频繁读取会造成自我认知剥离"]
            },
            "key_locations": [{
                "name": "档案城",
                "description": "保存灾变记忆的城市",
                "story_function": "主角调查和权力审查的核心舞台"
            }],
            "factions": [{
                "name": "审校局",
                "description": "管控记忆流通",
                "agenda": "公开维护秩序，暗中垄断灾变真相",
                "representative_characters": ["裴玄"]
            }],
            "history_timeline": [{
                "period": "星潮灾变",
                "event": "大批居民失去关键记忆",
                "impact": "催生档案城与审校制度"
            }],
            "culture_and_society": {
                "values": "稳定高于真实",
                "class_structure": "记忆完整者拥有更高社会信用",
                "daily_life": "居民以记忆凭证兑换工作、住所和医疗"
            },
            "conflict_sources": [{
                "name": "真相公开与秩序维持",
                "description": "公开灾变真相可能瓦解审校局统治",
                "stakes": "失败会让全城继续活在伪造记忆里"
            }],
            "mysteries_and_foreshadowing": [{
                "mystery": "林澈妹妹的失踪是否与第一份星潮档案有关",
                "early_clues": ["妹妹留下空白记忆匣", "裴玄回避旧档案编号"],
                "payoff_direction": "她是审校局隐藏第一批实验的关键证人"
            }],
            "genre_promises": ["记忆悬疑", "制度阴谋", "同伴背叛与救赎"]
        },
        "characters": [
            {
                "name": "林澈",
                "character_type": "protagonist",
                "gender": "男",
                "age": "27",
                "appearance": "黑发灰眼，穿旧档案员制服",
                "personality": "谨慎但执拗",
                "background": "灾变幸存档案员",
                "motivation": "找回妹妹失踪真相",
                "arc_summary": "从旁观记录者成长为公开真相的人",
                "relationship_to_protagonist": "本人",
                "arc_stages": {
                    "start": "逃避真相，只相信档案记录",
                    "core_wound": "妹妹失踪时自己选择遵守流程而非救人",
                    "desire": "找回妹妹并证明自己没有失职",
                    "need": "承认制度本身正在制造伤害",
                    "secret": "他曾亲手封存过妹妹的求救片段",
                    "voice_style": "短句、克制，常用档案术语描述情绪",
                    "turning_point": "发现自己的签名出现在禁档上",
                    "end": "承担公开真相的代价"
                }
            },
            {
                "name": "许灯",
                "character_type": "supporting",
                "gender": "女",
                "age": "25",
                "appearance": "短发，机械义眼",
                "personality": "直率机敏",
                "background": "黑市记忆修补师",
                "motivation": "偿还旧债",
                "arc_summary": "从逐利者转向主动保护他人",
                "relationship_to_protagonist": "盟友",
                "arc_stages": {
                    "start": "只为报酬修补记忆",
                    "core_wound": "曾因救人失败而拒绝长期承诺",
                    "desire": "还清债务后离开档案城",
                    "need": "重新相信自己能保护别人",
                    "secret": "她修补过裴玄删改的证词",
                    "voice_style": "语速快，带黑市俚语，用玩笑掩饰恐惧",
                    "turning_point": "主动留下帮助林澈破解禁档",
                    "end": "成为公开档案的共同守护者"
                }
            },
            {
                "name": "裴玄",
                "character_type": "antagonist",
                "gender": "男",
                "age": "42",
                "appearance": "白发，常戴银框眼镜",
                "personality": "温和而控制欲强",
                "background": "审校局首席",
                "motivation": "维持灾后秩序",
                "arc_summary": "从秩序守护者暴露为记忆垄断者",
                "relationship_to_protagonist": "对手",
                "arc_stages": {
                    "start": "以温和姿态维护审校秩序",
                    "core_wound": "亲历灾变混乱后恐惧失控",
                    "desire": "让所有人相信秩序比真相更重要",
                    "need": "面对自己用谎言延续灾难",
                    "secret": "他保留着星潮源头的完整记忆",
                    "voice_style": "礼貌、缓慢，常把威胁包装成建议",
                    "turning_point": "亲自抹除证人记忆失败",
                    "end": "在真相公开后失去审校局合法性"
                }
            }
        ],
        "relationships": [
            {
                "relationship_type": "rival",
                "character_a": "林澈",
                "character_b": "裴玄",
                "description": "真相公开与秩序维持的核心对立",
                "strength": 8,
                "evolution": {
                    "start": "试探",
                    "tension": "林澈越接近真相，裴玄越需要证明封锁是必要的",
                    "turning_points": ["裴玄归还残缺记忆匣", "林澈发现禁档签名"],
                    "hidden_truth": "裴玄知道林澈妹妹的真实去向",
                    "story_function": "把个人寻亲线推向制度真相线",
                    "end": "决裂"
                }
            }
        ],
        "chapter_outline": [],
        "needs_part_outlines": false,
        "total_chapters": 30,
        "chapters_per_part": 25
    })
    .to_string()
}
