export const formatConfigTestTime = (
  value: string,
  options: { withSeconds?: boolean } = {},
): string => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    ...(options.withSeconds ? { second: "2-digit" as const } : {}),
  });
};
