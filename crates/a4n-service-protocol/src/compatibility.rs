use serde::{Deserialize, Serialize};

use crate::{PROTOCOL_VERSION, expected_capability_version_for_service};

/// 协议/能力版本的闭区间。当前 v2 合同只接受 v2；未来做 additive 升级时
/// 只需在这里扩展范围，不要在每个 client/router 里复制判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRange {
    pub min: u32,
    pub max: u32,
}

impl VersionRange {
    pub const fn exact(version: u32) -> Self {
        Self {
            min: version,
            max: version,
        }
    }

    pub const fn contains(self, version: u32) -> bool {
        version >= self.min && version <= self.max
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceCompatibility {
    pub protocol: VersionRange,
    pub capability_major: VersionRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedServiceVersion {
    pub protocol_version: u32,
    pub capability_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentMode {
    LocalAtomicBundle,
    RemoteRolling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolChangeKind {
    AdditiveOptionalField,
    Breaking,
    FieldRemoval,
    SemanticChange,
}

// 同一 major 只允许在 request/response/health/event/error 顶层 envelope
// 增加可选字段。secret/schema/context、业务 payload、artifact 和 release
// manifest 仍保持严格解析；这些边界变化必须提升 major。

impl ProtocolChangeKind {
    pub const fn requires_major_bump(self) -> bool {
        !matches!(self, Self::AdditiveOptionalField)
    }
}

pub const SUPPORTED_PROTOCOL_RANGE: VersionRange = VersionRange::exact(PROTOCOL_VERSION);

pub fn service_compatibility(service: &str) -> Option<ServiceCompatibility> {
    let capability = expected_capability_version_for_service(service)?;
    Some(ServiceCompatibility {
        protocol: SUPPORTED_PROTOCOL_RANGE,
        capability_major: VersionRange::exact(capability_major(capability)?),
    })
}

pub fn service_compatibility_for_host(
    service: &str,
    host_protocol_version: Option<u32>,
) -> Option<ServiceCompatibility> {
    host_protocol_version
        .filter(|version| SUPPORTED_PROTOCOL_RANGE.contains(*version))
        .and_then(|_| service_compatibility(service))
}

/// 在 health 握手时选择双方实际可用的版本。
///
/// capability 版本采用 `<capability-name>-v<major>` 合同；同一 major 的
/// additive 字段可以滚动部署，换 major 或 capability 名称则稳定拒绝。
pub fn negotiate_service_version(
    service: &str,
    host_protocol_version: u32,
    sidecar_protocol_version: u32,
    expected_capability_version: Option<&str>,
    actual_capability_version: &str,
) -> Result<NegotiatedServiceVersion, VersionNegotiationError> {
    let compatibility = service_compatibility(service).unwrap_or(ServiceCompatibility {
        protocol: SUPPORTED_PROTOCOL_RANGE,
        capability_major: VersionRange::exact(0),
    });
    if !compatibility.protocol.contains(host_protocol_version)
        || !compatibility.protocol.contains(sidecar_protocol_version)
    {
        return Err(VersionNegotiationError::ProtocolOutOfRange {
            supported: compatibility.protocol,
            host: host_protocol_version,
            sidecar: sidecar_protocol_version,
        });
    }

    if let Some(expected) = expected_capability_version {
        let expected_name = capability_name(expected);
        let actual_name = capability_name(actual_capability_version);
        let actual_major = capability_major(actual_capability_version);
        if expected_name != actual_name
            || actual_major.is_none_or(|major| !compatibility.capability_major.contains(major))
        {
            return Err(VersionNegotiationError::CapabilityOutOfRange {
                expected: expected.to_string(),
                actual: actual_capability_version.to_string(),
                supported: compatibility.capability_major,
            });
        }
    }

    Ok(NegotiatedServiceVersion {
        protocol_version: sidecar_protocol_version,
        capability_version: actual_capability_version.to_string(),
    })
}

pub fn negotiate_deployment_version(
    mode: DeploymentMode,
    service: &str,
    host_service_version: &str,
    sidecar_service_version: &str,
    host_protocol_version: u32,
    sidecar_protocol_version: u32,
    expected_capability_version: Option<&str>,
    actual_capability_version: &str,
) -> Result<NegotiatedServiceVersion, VersionNegotiationError> {
    if matches!(mode, DeploymentMode::LocalAtomicBundle)
        && host_service_version != sidecar_service_version
    {
        return Err(VersionNegotiationError::MixedLocalBundle {
            host: host_service_version.to_string(),
            sidecar: sidecar_service_version.to_string(),
        });
    }
    negotiate_service_version(
        service,
        host_protocol_version,
        sidecar_protocol_version,
        expected_capability_version,
        actual_capability_version,
    )
}

pub fn capability_major(version: &str) -> Option<u32> {
    let suffix = version.rsplit_once("-v")?.1;
    let mut segments = suffix.split('.');
    let major = segments.next()?;
    if major.is_empty() || !major.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    if segments.any(|segment| {
        segment.is_empty() || !segment.chars().all(|character| character.is_ascii_digit())
    }) {
        return None;
    }
    major.parse().ok()
}

pub(crate) fn capability_name(version: &str) -> Option<&str> {
    version.rsplit_once("-v").map(|(name, _)| name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionNegotiationError {
    MixedLocalBundle {
        host: String,
        sidecar: String,
    },
    ProtocolOutOfRange {
        supported: VersionRange,
        host: u32,
        sidecar: u32,
    },
    CapabilityOutOfRange {
        expected: String,
        actual: String,
        supported: VersionRange,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_versions_negotiate() {
        let result = negotiate_service_version(
            "prompt",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v2"),
            "prompt-render-v2",
        )
        .expect("current versions should negotiate");
        assert_eq!(result.protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn health_compatibility_is_only_advertised_to_supported_hosts() {
        assert!(service_compatibility_for_host("prompt", Some(PROTOCOL_VERSION)).is_some());
        assert!(service_compatibility_for_host("prompt", None).is_none());
        assert!(service_compatibility_for_host("prompt", Some(PROTOCOL_VERSION + 1)).is_none());
    }

    #[test]
    fn additive_same_major_capability_is_accepted() {
        negotiate_service_version(
            "prompt",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v2"),
            "prompt-render-v2",
        )
        .expect("same capability major should remain compatible");
        negotiate_service_version(
            "prompt",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v2"),
            "prompt-render-v2.1",
        )
        .expect("same capability major with additive suffix should remain compatible");
    }

    #[test]
    fn old_protocol_and_breaking_capability_are_rejected() {
        assert!(matches!(
            negotiate_service_version(
                "prompt",
                PROTOCOL_VERSION,
                PROTOCOL_VERSION - 1,
                Some("prompt-render-v2"),
                "prompt-render-v2",
            ),
            Err(VersionNegotiationError::ProtocolOutOfRange { .. })
        ));
        assert!(matches!(
            negotiate_service_version(
                "prompt",
                PROTOCOL_VERSION,
                PROTOCOL_VERSION,
                Some("prompt-render-v2"),
                "prompt-render-v3",
            ),
            Err(VersionNegotiationError::CapabilityOutOfRange { .. })
        ));
    }

    #[test]
    fn local_bundle_is_atomic_but_remote_rollout_can_mix_service_builds() {
        assert!(matches!(
            negotiate_deployment_version(
                DeploymentMode::LocalAtomicBundle,
                "prompt",
                "2.1.0",
                "2.0.0",
                PROTOCOL_VERSION,
                PROTOCOL_VERSION,
                Some("prompt-render-v2"),
                "prompt-render-v2",
            ),
            Err(VersionNegotiationError::MixedLocalBundle { .. })
        ));
        negotiate_deployment_version(
            DeploymentMode::RemoteRolling,
            "prompt",
            "2.1.0",
            "2.0.0",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v2"),
            "prompt-render-v2",
        )
        .expect("remote rollout may mix service builds inside the negotiated range");
    }

    #[test]
    fn remote_rollout_and_rollback_version_matrix_is_directional_and_fail_closed() {
        for (host_build, sidecar_build, actual_capability) in [
            ("host-new", "sidecar-old", "prompt-render-v2"),
            ("host-old", "sidecar-new", "prompt-render-v2.1"),
            ("host-new", "sidecar-rollback", "prompt-render-v2"),
        ] {
            negotiate_deployment_version(
                DeploymentMode::RemoteRolling,
                "prompt",
                host_build,
                sidecar_build,
                PROTOCOL_VERSION,
                PROTOCOL_VERSION,
                Some("prompt-render-v2"),
                actual_capability,
            )
            .expect("compatible remote rollout or rollback should negotiate");
        }

        assert!(matches!(
            negotiate_deployment_version(
                DeploymentMode::RemoteRolling,
                "prompt",
                "host-new",
                "sidecar-breaking-rollback",
                PROTOCOL_VERSION,
                PROTOCOL_VERSION,
                Some("prompt-render-v2"),
                "prompt-render-v1",
            ),
            Err(VersionNegotiationError::CapabilityOutOfRange { .. })
        ));
    }

    #[test]
    fn only_additive_optional_fields_avoid_a_major_bump() {
        assert!(!ProtocolChangeKind::AdditiveOptionalField.requires_major_bump());
        for change in [
            ProtocolChangeKind::Breaking,
            ProtocolChangeKind::FieldRemoval,
            ProtocolChangeKind::SemanticChange,
        ] {
            assert!(change.requires_major_bump());
        }
    }

    #[test]
    fn malformed_capability_suffixes_fail_closed() {
        assert_eq!(capability_major("prompt-render-v2"), Some(2));
        assert_eq!(capability_major("prompt-render-v2.1.3"), Some(2));
        for version in [
            "prompt-render-v2garbage",
            "prompt-render-v2.",
            "prompt-render-v2..1",
            "prompt-render-v-two",
            "prompt-render-v",
            "prompt-render-v999999999999999999999999999999999999",
        ] {
            assert_eq!(
                capability_major(version),
                None,
                "{version} must be rejected"
            );
            assert_eq!(
                crate::service_for_capability_version(version),
                None,
                "{version} must not resolve to a service"
            );
        }
    }
}
