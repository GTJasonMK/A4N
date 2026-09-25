export function extractErrorMessage(err: unknown, defaultMessage: string): string {
  if (typeof err === "string") return err || defaultMessage;
  if (err instanceof Error) return err.message || defaultMessage;
  if (err && typeof err === "object") {
    const value = err as Record<string, unknown>;
    if (typeof value.message === "string") return value.message || defaultMessage;
  }
  return defaultMessage;
}
