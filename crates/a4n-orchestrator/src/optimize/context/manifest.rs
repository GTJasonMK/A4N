use serde_json::{Value, json};

pub(in crate::optimize) fn optimization_context_manifest(context: &Value) -> Value {
    let Some(object) = context.as_object() else {
        return json!({
            "sections": [],
            "counts": {},
            "section_details": [],
        });
    };
    let mut section_names = object.keys().cloned().collect::<Vec<_>>();
    section_names.sort();
    let mut counts = serde_json::Map::new();
    for key in [
        "part_outlines",
        "chapter_outlines",
        "character_profiles",
        "foreshadowing_index",
        "previous_chapters",
        "next_chapters",
    ] {
        if let Some(items) = object.get(key).and_then(Value::as_array) {
            counts.insert(key.to_string(), json!(items.len()));
        }
    }
    for key in [
        "current_chapter",
        "current_chapter_outline",
        "project_detail",
        "blueprint",
    ] {
        if object.contains_key(key) {
            counts.insert(key.to_string(), json!(1));
        }
    }
    let section_details = section_names
        .iter()
        .filter_map(|name| optimization_context_section_detail(name, counts.get(name)))
        .collect::<Vec<_>>();

    json!({
        "sections": section_names,
        "counts": counts,
        "section_details": section_details,
    })
}

fn optimization_context_section_detail(name: &str, count: Option<&Value>) -> Option<Value> {
    let description = match name {
        "current_chapter" => {
            "当前章节元信息；get_project_information 读取后会返回当前章节正文和元信息"
        }
        "current_chapter_outline" => "当前章节章纲、节拍、角色、伏笔和字数目标",
        "project_detail" => "项目标题、类型、简介、目标章节数和原始项目配置",
        "blueprint" => "项目蓝图、主要角色关系和整体设定",
        "part_outlines" => "分卷/分部大纲，用于判断章节在整体结构中的位置",
        "chapter_outlines" => "全书章节大纲列表，用于判断前后承接和长期铺垫",
        "character_profiles" => "角色档案，用于判断动机、性格、口吻和关系一致性",
        "foreshadowing_index" => "伏笔索引，用于判断埋设、回收和遗漏",
        "previous_chapters" => "当前章之前的章节摘要，用于判断上下文承接",
        "next_chapters" => "当前章之后的章节摘要，用于判断后续铺垫和走向",
        _ => return None,
    };
    Some(json!({
        "name": name,
        "count": count.cloned(),
        "description": description,
    }))
}
