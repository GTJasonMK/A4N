use std::collections::BTreeSet;

use super::{
    AdaptationBeat, AdaptationBeatRef, AdaptationOrigin, AdaptationPlanV1, AdaptationUnitRef,
};

impl AdaptationUnitRef {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_chapter_id <= 0 {
            return Err("原文章节 ID 必须大于 0".into());
        }
        required(&self.unit_id, "unit_id")
    }
}

impl AdaptationBeatRef {
    pub fn validate(&self) -> Result<(), String> {
        required(&self.unit_id, "unit_id")?;
        required(&self.beat_id, "beat_id")
    }
}

impl AdaptationPlanV1 {
    /// Unit-level citations aggregate the evidence cited by their beats.
    pub fn complete_unit_source_span_ids_from_beats(&mut self) {
        for unit in &mut self.units {
            let mut seen = unit
                .source_span_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            let beat_source_span_ids = unit
                .beats
                .iter()
                .flat_map(|beat| &beat.source_span_ids)
                .filter(|id| seen.insert((*id).clone()))
                .cloned()
                .collect::<Vec<_>>();
            unit.source_span_ids.extend(beat_source_span_ids);
        }
    }

    pub fn validate(&self, source_span_ids: &BTreeSet<String>) -> Result<(), String> {
        if let Some(page_count) = self.page_count {
            if self.target != crate::media_authoring::AdaptationTarget::Manga {
                return Err("只有漫画改编计划可以声明 page_count".into());
            }
            if !(1..=crate::manga::MANGA_PAGE_SLOT_LIMIT as u32).contains(&page_count) {
                return Err(format!(
                    "page_count 必须在 1-{} 之间",
                    crate::manga::MANGA_PAGE_SLOT_LIMIT
                ));
            }
        }
        required(&self.intention, "intention")?;
        if self.units.is_empty() {
            return Err("改编计划至少需要一个叙事单元".into());
        }
        let mut units = BTreeSet::new();
        for unit in &self.units {
            required(&unit.id, "unit.id")?;
            if !units.insert(&unit.id) {
                return Err(format!("改编单元 ID 重复: {}", unit.id));
            }
            required(&unit.title, "unit.title")?;
            required(&unit.objective, "unit.objective")?;
            validate_adaptation_evidence(unit.origin, &unit.source_span_ids, source_span_ids)?;
            if unit.beats.is_empty() {
                return Err(format!("改编单元 {} 缺少节拍", unit.id));
            }
            let mut beats = BTreeSet::new();
            for beat in &unit.beats {
                required(&beat.id, "beat.id")?;
                if !beats.insert(&beat.id) {
                    return Err(format!("改编单元 {} 的节拍 ID 重复: {}", unit.id, beat.id));
                }
                required(&beat.action, "beat.action")?;
                validate_adaptation_evidence(beat.origin, &beat.source_span_ids, source_span_ids)?;
                if beat
                    .source_span_ids
                    .iter()
                    .any(|id| !unit.source_span_ids.contains(id))
                {
                    return Err(format!("单元 {} 的原文依据未覆盖节拍 {}", unit.id, beat.id));
                }
            }
        }
        for decision in &self.decisions {
            required(&decision.description, "decision.description")?;
            required(&decision.reason, "decision.reason")?;
            validate_adaptation_evidence(
                AdaptationOrigin::Historical,
                &decision.source_span_ids,
                source_span_ids,
            )?;
        }
        Ok(())
    }

    pub fn validate_manga_page_count(&self) -> Result<u32, String> {
        if self.target != crate::media_authoring::AdaptationTarget::Manga {
            return Err("改编计划目标不是漫画".into());
        }
        self.page_count
            .filter(|count| (1..=crate::manga::MANGA_PAGE_SLOT_LIMIT as u32).contains(count))
            .ok_or_else(|| {
                format!(
                    "漫画改编计划必须设置 1-{} 页",
                    crate::manga::MANGA_PAGE_SLOT_LIMIT
                )
            })
    }

    pub fn beat(&self, reference: &AdaptationBeatRef) -> Option<&AdaptationBeat> {
        self.units
            .iter()
            .find(|unit| unit.id == reference.unit_id)?
            .beats
            .iter()
            .find(|beat| beat.id == reference.beat_id)
    }
}

fn required(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} 不能为空"));
    }
    Ok(())
}

pub fn validate_adaptation_evidence(
    origin: AdaptationOrigin,
    spans: &[String],
    available: &BTreeSet<String>,
) -> Result<(), String> {
    if matches!(origin, AdaptationOrigin::Source | AdaptationOrigin::Adapted) && spans.is_empty() {
        return Err("来自原文的改编内容必须保留原文引用".into());
    }
    if origin == AdaptationOrigin::Added && !spans.is_empty() {
        return Err("新增改编内容不能声明为原文事实".into());
    }
    let mut seen = BTreeSet::new();
    for span in spans {
        if !available.contains(span) || !seen.insert(span) {
            return Err(format!("改编原文引用不存在或重复: {span}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_authoring::{AdaptationBeatKind, AdaptationImportance, AdaptationTarget};

    #[test]
    fn adaptation_identity_survives_reordering_and_rejects_fabricated_sources() {
        let mut plan = AdaptationPlanV1 {
            schema_version: crate::media_authoring::AdaptationPlanVersion::V1,
            target: AdaptationTarget::Manga,
            page_count: Some(8),
            intention: "保留发现真相的因果".into(),
            decisions: vec![],
            units: vec![crate::media_authoring::AdaptationUnit {
                id: "discovery".into(),
                origin: AdaptationOrigin::Source,
                title: "发现信件".into(),
                location: None,
                participants: vec![],
                dramatic_question: "信是谁寄的".into(),
                objective: "找到寄件人".into(),
                obstacle: "落款被撕掉".into(),
                entry_state: "尚未读信".into(),
                exit_state: "发现缺失落款".into(),
                source_span_ids: vec!["p1".into()],
                beats: vec![AdaptationBeat {
                    id: "letter".into(),
                    kind: AdaptationBeatKind::Revelation,
                    is_climax: true,
                    origin: AdaptationOrigin::Source,
                    importance: AdaptationImportance::Required,
                    action: "翻到信纸背面".into(),
                    reaction: None,
                    dialogue_intent: None,
                    entry_state: "信纸正面朝上".into(),
                    exit_state: "露出被撕去的落款".into(),
                    source_span_ids: vec!["p1".into()],
                }],
            }],
        };
        let available = BTreeSet::from(["p1".into()]);
        plan.validate(&available).unwrap();
        let reference = AdaptationBeatRef {
            unit_id: "discovery".into(),
            beat_id: "letter".into(),
        };
        let mut added = plan.units[0].clone();
        added.id = "reaction".into();
        plan.units.insert(0, added);
        assert_eq!(plan.beat(&reference).unwrap().action, "翻到信纸背面");
        plan.validate(&available).unwrap();
        plan.units[0].beats[0].origin = AdaptationOrigin::Added;
        assert!(plan.validate(&available).unwrap_err().contains("新增改编"));
        plan.units[0].beats[0].origin = AdaptationOrigin::Source;
        plan.units[0].beats[0].source_span_ids = vec!["missing".into()];
        assert!(plan.validate(&available).unwrap_err().contains("不存在"));
    }
}
