export const configCreateLabel = (title: string): string =>
  /^[A-Za-z]/.test(title) ? `新增 ${title}` : `新增${title}`;
