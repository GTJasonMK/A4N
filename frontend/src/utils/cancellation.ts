export const isCancelledOperation = (message: string): boolean => {
  const lower = message.toLowerCase();
  return (
    lower.includes("cancelled")
    || lower.includes("canceled")
    || lower.includes("stopped")
    || message.includes("已取消")
    || message.includes("已停止")
  );
};
