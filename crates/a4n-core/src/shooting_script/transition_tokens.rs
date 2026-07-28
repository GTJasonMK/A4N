//! 拍摄剧本镜头转场规范 token 的唯一识别与边界决议合同。
//!
//! 视频执行、拍摄剧本落库校验必须复用本模块，禁止各自实现自然语言别名或静默默认。

use super::contracts::ShootingTransitionTypeV1;

/// 从自由文本中识别规范转场 token 的错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionTokenError {
    /// 文本中同时出现多种规范 token。
    Ambiguous {
        matched: Vec<&'static str>,
        original: String,
    },
    /// 文本非空但不含任何规范 token。
    Missing { original: String },
}

impl std::fmt::Display for TransitionTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ambiguous { matched, original } => write!(
                f,
                "必须只指定一种确定手法，实际同时包含：{}；原文：{original}",
                matched.join("、")
            ),
            Self::Missing { original } if original.is_empty() => {
                write!(
                    f,
                    "不能为空，且须包含规范转场 token（如 hard_cut：…）"
                )
            }
            Self::Missing { original } => {
                write!(f, "缺少规范转场值：{original:?}")
            }
        }
    }
}

/// 相邻镜头边界决议错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionBoundaryError {
    TransitionOut(TransitionTokenError),
    TransitionIn(TransitionTokenError),
    Mismatch {
        transition_out: ShootingTransitionTypeV1,
        transition_in: ShootingTransitionTypeV1,
    },
    ContractConflict {
        contract: ShootingTransitionTypeV1,
        shot: ShootingTransitionTypeV1,
    },
}

impl std::fmt::Display for TransitionBoundaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransitionOut(error) => write!(f, "transition_out {error}"),
            Self::TransitionIn(error) => write!(f, "transition_in {error}"),
            Self::Mismatch {
                transition_out,
                transition_in,
            } => write!(
                f,
                "transition_out={} 与 transition_in={} 不一致",
                transition_out.as_str(),
                transition_in.as_str()
            ),
            Self::ContractConflict { contract, shot } => write!(
                f,
                "场景转场 {} 与镜头转场 {} 冲突",
                contract.as_str(),
                shot.as_str()
            ),
        }
    }
}

impl TransitionBoundaryError {
    /// 面向视频/编辑链路的镜头边界错误文案（唯一出口，避免各层各写一套）。
    pub fn shot_boundary_message(
        &self,
        from_shot_id: &str,
        to_shot_id: &str,
        transition_out: &str,
        transition_in: &str,
    ) -> String {
        let prefix = format!("拍摄剧本镜头 {from_shot_id} → {to_shot_id}");
        match self {
            Self::TransitionOut(TransitionTokenError::Missing { .. }) => format!(
                "{prefix} 的 transition_out 缺少规范转场值：{transition_out:?}；请在正文页“剧本改编”中重新生成或编辑该镜头边界"
            ),
            Self::TransitionIn(TransitionTokenError::Missing { .. }) => format!(
                "{prefix} 的 transition_in 缺少规范转场值：{transition_in:?}；请在正文页“剧本改编”中重新生成或编辑该镜头边界"
            ),
            Self::TransitionOut(error) => format!("{prefix} 的 transition_out {error}"),
            Self::TransitionIn(error) => format!("{prefix} 的 transition_in {error}"),
            Self::Mismatch {
                transition_out: out_kind,
                transition_in: in_kind,
            } => format!(
                "{prefix} 的 transition_out={} 与 transition_in={} 不一致",
                out_kind.as_str(),
                in_kind.as_str(),
            ),
            Self::ContractConflict { contract, shot } => format!(
                "{prefix} 的场景转场 {} 与镜头转场 {} 冲突",
                contract.as_str(),
                shot.as_str(),
            ),
        }
    }
}

impl ShootingTransitionTypeV1 {
    /// 全部规范转场手法（识别顺序无关，按独立 token 匹配）。
    pub const ALL: [Self; 10] = [
        Self::Continuous,
        Self::HardCut,
        Self::MatchCut,
        Self::WhipPan,
        Self::ActionWipe,
        Self::SoundBridge,
        Self::Dissolve,
        Self::Fade,
        Self::FocusTransition,
        Self::Morph,
    ];

    /// 在自由文本中识别 0 或 1 个规范转场 token。
    ///
    /// - 空串 → `Ok(None)`
    /// - 恰好一种 → `Ok(Some(...))`
    /// - 多种 → `Err(Ambiguous)`
    ///
    /// 故意不接受中文别名（如「硬切」），避免与下游机器合同漂移。
    pub fn recognize_in_text(value: &str) -> Result<Option<Self>, TransitionTokenError> {
        let normalized = value.trim().to_lowercase();
        if normalized.is_empty() {
            return Ok(None);
        }
        let matched = Self::ALL
            .iter()
            .copied()
            .filter(|kind| contains_transition_token(&normalized, kind.as_str()))
            .collect::<Vec<_>>();
        if matched.len() > 1 {
            return Err(TransitionTokenError::Ambiguous {
                matched: matched.iter().map(|kind| kind.as_str()).collect(),
                original: value.trim().to_string(),
            });
        }
        Ok(matched.first().copied())
    }

    /// 要求文本中恰好包含一种规范转场 token。
    pub fn require_in_text(value: &str) -> Result<Self, TransitionTokenError> {
        match Self::recognize_in_text(value)? {
            Some(kind) => Ok(kind),
            None => Err(TransitionTokenError::Missing {
                original: value.trim().to_string(),
            }),
        }
    }

    /// 决议相邻镜头边界的唯一转场手法，与视频执行侧规则一致。
    ///
    /// - `transition_out` 与 `transition_in` 必须指向同一种手法
    /// - `sound_bridge` 合同可与非声音桥的视觉手法并存（取视觉手法）
    /// - 镜头侧为 `sound_bridge` 而合同是其它类型时，取合同类型
    /// - 其它情况下合同必须与镜头手法一致
    pub fn resolve_boundary(
        transition_out: &str,
        transition_in: &str,
        contract_type: Option<Self>,
    ) -> Result<Self, TransitionBoundaryError> {
        let transition_out_type = Self::require_in_text(transition_out)
            .map_err(TransitionBoundaryError::TransitionOut)?;
        let transition_in_type =
            Self::require_in_text(transition_in).map_err(TransitionBoundaryError::TransitionIn)?;
        if transition_out_type != transition_in_type {
            return Err(TransitionBoundaryError::Mismatch {
                transition_out: transition_out_type,
                transition_in: transition_in_type,
            });
        }
        if contract_type == Some(Self::SoundBridge) && transition_out_type != Self::SoundBridge {
            return Ok(transition_out_type);
        }
        if transition_out_type == Self::SoundBridge
            && let Some(contract_type) = contract_type
            && contract_type != Self::SoundBridge
        {
            return Ok(contract_type);
        }
        if let Some(contract_type) = contract_type
            && contract_type != transition_out_type
        {
            return Err(TransitionBoundaryError::ContractConflict {
                contract: contract_type,
                shot: transition_out_type,
            });
        }
        Ok(contract_type.unwrap_or(transition_out_type))
    }
}

fn contains_transition_token(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(start, matched)| {
        let before = text[..start].chars().next_back();
        let after = text[start + matched.len()..].chars().next();
        before.is_none_or(|value| !is_identifier_char(value))
            && after.is_none_or(|value| !is_identifier_char(value))
    })
}

fn is_identifier_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_canonical_token_in_prose() {
        let kind = ShootingTransitionTypeV1::recognize_in_text("hard_cut：人物抬眼时切出")
            .expect("recognize");
        assert_eq!(kind, Some(ShootingTransitionTypeV1::HardCut));
    }

    #[test]
    fn rejects_natural_language_alias() {
        let kind = ShootingTransitionTypeV1::recognize_in_text("人物抬眼时硬切进入洗漱台")
            .expect("recognize");
        assert_eq!(kind, None);
    }

    #[test]
    fn rejects_embedded_token_in_identifier() {
        let kind = ShootingTransitionTypeV1::recognize_in_text("not_hard_cut：进入洗漱台")
            .expect("recognize");
        assert_eq!(kind, None);
    }

    #[test]
    fn rejects_ambiguous_methods() {
        let err = ShootingTransitionTypeV1::recognize_in_text("hard_cut 或 dissolve")
            .expect_err("ambiguous");
        assert!(matches!(err, TransitionTokenError::Ambiguous { .. }));
    }

    #[test]
    fn resolve_boundary_accepts_sound_bridge_contract_with_visual_hard_cut() {
        let kind = ShootingTransitionTypeV1::resolve_boundary(
            "hard_cut：闹钟声延续到切点",
            "hard_cut：切入洗漱镜前",
            Some(ShootingTransitionTypeV1::SoundBridge),
        )
        .expect("boundary");
        assert_eq!(kind, ShootingTransitionTypeV1::HardCut);
    }

    #[test]
    fn shot_boundary_message_is_stable_for_missing_token() {
        let err =
            ShootingTransitionTypeV1::resolve_boundary("人物抬眼时硬切", "hard_cut：切入", None)
                .expect_err("missing token");
        let message = err.shot_boundary_message(
            "unit_1_shot_1",
            "unit_1_shot_2",
            "人物抬眼时硬切",
            "hard_cut：切入",
        );
        assert!(message.contains("transition_out 缺少规范转场值"));
        assert!(message.contains("unit_1_shot_1"));
    }
}
