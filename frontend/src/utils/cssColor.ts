export type Rgb = readonly [number, number, number];

export function parseCssRgb(rawValue: string, fallback: Rgb): Rgb {
  const raw = rawValue.trim();
  const rgbMatch = raw.match(/^rgba?\(([^)]+)\)$/i);
  const channels = (rgbMatch?.[1] ?? raw)
    .replace(/,/g, " ")
    .replace(/\//g, " ")
    .split(/\s+/)
    .map((part) => Number.parseFloat(part));
  if (channels.length >= 3 && channels.slice(0, 3).every(Number.isFinite)) {
    return [channels[0], channels[1], channels[2]];
  }

  const hex = raw.match(/^#?([0-9a-f]{6})$/i)?.[1];
  return hex
    ? [
        Number.parseInt(hex.slice(0, 2), 16),
        Number.parseInt(hex.slice(2, 4), 16),
        Number.parseInt(hex.slice(4, 6), 16),
      ]
    : fallback;
}

export function readCssRgbVariable(
  style: Pick<CSSStyleDeclaration, "getPropertyValue">,
  name: string,
  fallback: Rgb,
): Rgb {
  return parseCssRgb(style.getPropertyValue(name), fallback);
}

export function rgb([r, g, b]: Rgb): string {
  return `rgb(${r}, ${g}, ${b})`;
}

export function rgba([r, g, b]: Rgb, alpha: number): string {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
