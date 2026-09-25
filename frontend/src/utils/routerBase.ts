export function routerBasename(): string | undefined {
  const value = import.meta.env.VITE_A4N_BASE_PATH?.trim();
  if (!value || value === "/") return undefined;
  return value.startsWith("/")
    ? value.replace(/\/$/, "")
    : `/${value.replace(/\/$/, "")}`;
}
