impl super::GalgamePipelineBeatRefV4 {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_chapter_id <= 0 {
            return Err("source_chapter_id 必须大于 0".into());
        }
        crate::media_authoring::AdaptationBeatRef {
            unit_id: self.unit_id.clone(),
            beat_id: self.beat_id.clone(),
        }
        .validate()
    }

    pub fn unit_ref(&self) -> super::GalgameAdaptationUnitRef {
        super::GalgameAdaptationUnitRef {
            source_chapter_id: self.source_chapter_id,
            unit_id: self.unit_id.clone(),
        }
    }
}

impl super::GalgameLinePresentationDocumentV3 {
    pub fn validate_source_identity(&self, text_anchor_id: Option<i64>) -> Result<(), String> {
        use crate::media_authoring::AdaptationOrigin;
        match self.origin {
            AdaptationOrigin::Source | AdaptationOrigin::Adapted if self.beat_ref.is_none() => {
                return Err("原文或改编演出必须引用本地节拍".into());
            }
            AdaptationOrigin::Added if text_anchor_id.is_some() => {
                return Err("新增演出不能冒充原文依据".into());
            }
            _ => {}
        }
        for reference in self.beat_ref.iter().chain(self.after_beat_ref.iter()) {
            reference.validate()?;
        }
        Ok(())
    }

    pub fn validate_adaptation(
        &self,
        plan: &crate::media_authoring::AdaptationPlanV1,
        chapter_id: i64,
        unit_id: &str,
        text_anchor_id: Option<i64>,
    ) -> Result<(), String> {
        self.validate_source_identity(text_anchor_id)?;
        for reference in self.beat_ref.iter().chain(self.after_beat_ref.iter()) {
            if reference.source_chapter_id != chapter_id
                || reference.unit_id != unit_id
                || plan
                    .beat(&crate::media_authoring::AdaptationBeatRef {
                        unit_id: reference.unit_id.clone(),
                        beat_id: reference.beat_id.clone(),
                    })
                    .is_none()
            {
                return Err("演出引用的节拍不属于该场景的本地改编版本".into());
            }
        }
        if matches!(
            self.origin,
            crate::media_authoring::AdaptationOrigin::Source
                | crate::media_authoring::AdaptationOrigin::Adapted
        ) && text_anchor_id.is_none()
            && let Some(reference) = &self.beat_ref
            && plan
                .beat(&crate::media_authoring::AdaptationBeatRef {
                    unit_id: reference.unit_id.clone(),
                    beat_id: reference.beat_id.clone(),
                })
                .is_some_and(|beat| beat.source_span_ids.is_empty())
        {
            return Err("原文或改编演出的本地节拍缺少原文依据".into());
        }
        Ok(())
    }

    pub fn directing_presentation(&self) -> super::GalgameLinePresentationV2 {
        super::GalgameLinePresentationV2 {
            schema_version: super::GalgameLinePresentationSchemaVersion::V2,
            visible_cast: self.visible_cast.clone(),
            enter: self.enter.clone(),
            exit: self.exit.clone(),
            focus_character_profile_id: self.focus_character_profile_id,
            hold: self.hold,
            player_knowledge: self.player_knowledge.clone(),
        }
    }
}

impl From<super::GalgameLinePresentationV2> for super::GalgameLinePresentationDocumentV3 {
    fn from(presentation: super::GalgameLinePresentationV2) -> Self {
        Self {
            schema_version: super::GalgameLineDocumentSchemaVersion::V3,
            origin: crate::media_authoring::AdaptationOrigin::Added,
            beat_ref: None,
            visible_cast: presentation.visible_cast,
            enter: presentation.enter,
            exit: presentation.exit,
            focus_character_profile_id: presentation.focus_character_profile_id,
            hold: presentation.hold,
            player_knowledge: presentation.player_knowledge,
            cg: None,
            cg_note: None,
            cg_importance: None,
            transition: None,
            branch: None,
            branch_id: None,
            returns_to_scene_index: None,
            after_beat_ref: None,
        }
    }
}
