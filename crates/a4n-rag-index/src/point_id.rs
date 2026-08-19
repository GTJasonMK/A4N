//! 业务 chunk_id（任意 UTF-8）→ Qdrant PointId（UUID v5）。

use uuid::Uuid;

/// 固定命名空间，勿改（改则全部 point id 失效，须 rebuild）。
const A4N_RAG_POINT_NAMESPACE: Uuid = Uuid::from_bytes([
    0xa4, 0x4e, 0x52, 0x41, 0x47, 0x49, 0x44, 0x00, // "A4NRAGID\0"
    0xb1, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
]);

/// 将稳定业务 id 映射为 Qdrant 可用的 UUID。
pub(super) fn chunk_id_to_point_uuid(chunk_id: &str) -> Uuid {
    Uuid::new_v5(&A4N_RAG_POINT_NAMESPACE, chunk_id.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_v5_is_stable() {
        let a = chunk_id_to_point_uuid("chapter:1:9:p2");
        let b = chunk_id_to_point_uuid("chapter:1:9:p2");
        assert_eq!(a, b);
        assert_ne!(
            chunk_id_to_point_uuid("chapter:1:9:p2"),
            chunk_id_to_point_uuid("chapter:1:9:p3")
        );
    }
}
