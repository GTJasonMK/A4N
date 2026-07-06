pub(super) fn is_cancelled_optimize_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("cancelled") || message.contains("已取消") || message.contains("已停止")
}
