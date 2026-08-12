use a4n_db::generation::{GenerationTaskListCursor, canonical_json_bytes};
use a4n_service_protocol::JS_SAFE_INTEGER_I64;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use uuid::{Uuid, Version};

const HISTORY_CURSOR_VERSION: &str = "v1";
pub const MAX_GENERATION_HISTORY_CURSOR_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationCursorError(String);

impl GenerationCursorError {
    fn invalid(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for GenerationCursorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for GenerationCursorError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GenerationHistoryCursorPayload {
    updated_at_ms: i64,
    task_id: String,
}

pub fn encode_generation_history_cursor(
    cursor: &GenerationTaskListCursor,
) -> Result<String, GenerationCursorError> {
    validate_cursor(cursor)?;
    let payload = canonical_json_bytes(&GenerationHistoryCursorPayload {
        updated_at_ms: cursor.updated_at_ms,
        task_id: cursor.task_id.clone(),
    })
    .map_err(|error| GenerationCursorError::invalid(format!("cursor JSON 编码失败: {error}")))?;
    let checksum = Sha256::digest(&payload);
    let encoded = format!(
        "{HISTORY_CURSOR_VERSION}.{}.{}",
        URL_SAFE_NO_PAD.encode(&payload),
        URL_SAFE_NO_PAD.encode(checksum)
    );
    if encoded.len() > MAX_GENERATION_HISTORY_CURSOR_BYTES {
        return Err(GenerationCursorError::invalid("cursor 超过 512 字节"));
    }
    Ok(encoded)
}

pub fn decode_generation_history_cursor(
    value: &str,
) -> Result<GenerationTaskListCursor, GenerationCursorError> {
    if value.is_empty() || value.len() > MAX_GENERATION_HISTORY_CURSOR_BYTES {
        return Err(GenerationCursorError::invalid(
            "cursor 长度必须在 1..=512 字节",
        ));
    }
    let mut parts = value.split('.');
    let version = parts.next();
    let payload_text = parts.next();
    let checksum_text = parts.next();
    if version != Some(HISTORY_CURSOR_VERSION)
        || payload_text.is_none()
        || checksum_text.is_none()
        || parts.next().is_some()
    {
        return Err(GenerationCursorError::invalid(
            "cursor 必须使用 v1.payload.checksum",
        ));
    }
    let payload_text = payload_text.expect("checked above");
    let checksum_text = checksum_text.expect("checked above");
    let payload = decode_base64url(payload_text, "payload")?;
    let checksum = decode_base64url(checksum_text, "checksum")?;
    if checksum.len() != 32 || checksum.as_slice() != Sha256::digest(&payload).as_slice() {
        return Err(GenerationCursorError::invalid("cursor checksum 无效"));
    }
    let decoded: GenerationHistoryCursorPayload = serde_json::from_slice(&payload)
        .map_err(|error| GenerationCursorError::invalid(format!("cursor JSON 无效: {error}")))?;
    let canonical = canonical_json_bytes(&decoded).map_err(|error| {
        GenerationCursorError::invalid(format!("cursor JSON 编码失败: {error}"))
    })?;
    if canonical != payload {
        return Err(GenerationCursorError::invalid(
            "cursor payload 不是 RFC8785 JSON",
        ));
    }
    let cursor = GenerationTaskListCursor {
        updated_at_ms: decoded.updated_at_ms,
        task_id: decoded.task_id,
    };
    validate_cursor(&cursor)?;
    Ok(cursor)
}

fn decode_base64url(value: &str, label: &str) -> Result<Vec<u8>, GenerationCursorError> {
    let decoded = URL_SAFE_NO_PAD.decode(value).map_err(|_| {
        GenerationCursorError::invalid(format!("cursor {label} 不是 base64url-no-pad"))
    })?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return Err(GenerationCursorError::invalid(format!(
            "cursor {label} 不是规范 base64url-no-pad"
        )));
    }
    Ok(decoded)
}

fn validate_cursor(cursor: &GenerationTaskListCursor) -> Result<(), GenerationCursorError> {
    if !(0..=JS_SAFE_INTEGER_I64).contains(&cursor.updated_at_ms) {
        return Err(GenerationCursorError::invalid(
            "cursor updated_at_ms 超出 JavaScript safe integer",
        ));
    }
    let task_id = Uuid::parse_str(&cursor.task_id)
        .map_err(|_| GenerationCursorError::invalid("cursor task_id 必须是 UUID v4"))?;
    if task_id.get_version() != Some(Version::Random) || task_id.to_string() != cursor.task_id {
        return Err(GenerationCursorError::invalid(
            "cursor task_id 必须是小写 UUID v4",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_cursor_round_trips_and_rejects_tampering() {
        let cursor = GenerationTaskListCursor {
            updated_at_ms: 1_750_000_000_123,
            task_id: "67263d6c-7557-4d2f-a75f-4cd5b1c1836a".to_string(),
        };
        let encoded = encode_generation_history_cursor(&cursor).unwrap();
        assert!(!encoded.contains('='));
        assert_eq!(decode_generation_history_cursor(&encoded).unwrap(), cursor);

        let mut tampered = encoded.into_bytes();
        let index = tampered.iter().position(|byte| *byte == b'.').unwrap() + 2;
        tampered[index] = if tampered[index] == b'A' { b'B' } else { b'A' };
        assert!(decode_generation_history_cursor(std::str::from_utf8(&tampered).unwrap()).is_err());
    }
}
