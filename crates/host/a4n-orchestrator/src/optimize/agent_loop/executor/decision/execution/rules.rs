use super::super::super::AppError;

pub(super) fn agent_loop_call_order(step: usize) -> Result<i32, AppError> {
    i32::try_from(step).map_err(|_| {
        AppError::InvalidArgument(format!("优化 agent step 超出 i32 call_order 范围：{step}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_loop_call_order_accepts_i32_max() {
        assert_eq!(
            agent_loop_call_order(i32::MAX as usize).expect("i32 max must fit"),
            i32::MAX
        );
    }

    #[test]
    fn agent_loop_call_order_rejects_overflow() {
        let err = agent_loop_call_order(usize::MAX).expect_err("overflow must fail");
        assert_eq!(
            err.to_string(),
            format!(
                "参数无效：优化 agent step 超出 i32 call_order 范围：{}",
                usize::MAX
            )
        );
    }
}
