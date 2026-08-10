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

const SUPPORTED_PROTOCOL_RANGE: VersionRange = VersionRange::exact(PROTOCOL_VERSION);

pub fn service_compatibility(service: &str) -> Option<ServiceCompatibility> {
    let capability = expected_capability_version_for_service(service)?;
    Some(ServiceCompatibility {
        protocol: SUPPORTED_PROTOCOL_RANGE,
        capability_major: VersionRange::exact(capability_major(capability)?),
    })
}

pub(crate) fn service_compatibility_for_host(
    service: &str,
    host_protocol_version: Option<u32>,
) -> Option<ServiceCompatibility> {
    host_protocol_version
        .filter(|version| SUPPORTED_PROTOCOL_RANGE.contains(*version))
        .and_then(|_| service_compatibility(service))
}

/// 在 health 握手时校验双方协议与 capability 主版本。
///
/// capability 版本采用 `<capability-name>-v<major>` 合同；同一 major 的
/// additive 字段可以滚动部署，换 major 或 capability 名称则稳定拒绝。
pub(crate) fn validate_service_version(
    service: &str,
    host_protocol_version: u32,
    sidecar_protocol_version: u32,
    expected_capability_version: Option<&str>,
    actual_capability_version: &str,
) -> Result<(), VersionNegotiationError> {
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

    Ok(())
}

pub(crate) fn capability_major(version: &str) -> Option<u32> {
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
pub(crate) enum VersionNegotiationError {
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
    fn current_versions_validate() {
        validate_service_version(
            "prompt",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v4"),
            "prompt-render-v4",
        )
        .expect("current versions should negotiate");
    }

    #[test]
    fn health_compatibility_is_only_advertised_to_supported_hosts() {
        assert!(service_compatibility_for_host("prompt", Some(PROTOCOL_VERSION)).is_some());
        assert!(service_compatibility_for_host("prompt", None).is_none());
        assert!(service_compatibility_for_host("prompt", Some(PROTOCOL_VERSION + 1)).is_none());
    }

    #[test]
    fn additive_same_major_capability_is_accepted() {
        validate_service_version(
            "prompt",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v4"),
            "prompt-render-v4",
        )
        .expect("same capability major should remain compatible");
        validate_service_version(
            "prompt",
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
            Some("prompt-render-v4"),
            "prompt-render-v4.1",
        )
        .expect("same capability major with additive suffix should remain compatible");
    }

    #[test]
    fn old_protocol_and_breaking_capability_are_rejected() {
        assert!(matches!(
            validate_service_version(
                "prompt",
                PROTOCOL_VERSION,
                PROTOCOL_VERSION - 1,
                Some("prompt-render-v4"),
                "prompt-render-v4",
            ),
            Err(VersionNegotiationError::ProtocolOutOfRange { .. })
        ));
        assert!(matches!(
            validate_service_version(
                "prompt",
                PROTOCOL_VERSION,
                PROTOCOL_VERSION,
                Some("prompt-render-v4"),
                "prompt-render-v5",
            ),
            Err(VersionNegotiationError::CapabilityOutOfRange { .. })
        ));
    }

    #[test]
    fn malformed_capability_suffixes_fail_closed() {
        assert_eq!(capability_major("prompt-render-v3"), Some(3));
        assert_eq!(capability_major("prompt-render-v3.1.3"), Some(3));
        for version in [
            "prompt-render-v3garbage",
            "prompt-render-v3.",
            "prompt-render-v3..1",
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
