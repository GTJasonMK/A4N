const MICROS_PER_UNIT = 1_000_000n;

export function formatMicros(value: string, locale: string): string {
  const micros = BigInt(value);
  const magnitude = micros < 0n ? -micros : micros;
  const whole = magnitude / MICROS_PER_UNIT;
  const remainder = (magnitude % MICROS_PER_UNIT).toString().padStart(6, "0");
  const trimmedFraction = remainder.replace(/0+$/, "");
  const fraction = trimmedFraction.length < 2
    ? trimmedFraction.padEnd(2, "0")
    : trimmedFraction;
  const integerFormatter = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });
  const digitFormatter = new Intl.NumberFormat(locale, { useGrouping: false });
  const decimalSeparator = new Intl.NumberFormat(locale, { minimumFractionDigits: 1 })
    .formatToParts(1)
    .find((part) => part.type === "decimal")!.value;
  const minusSign = micros < 0n
    ? new Intl.NumberFormat(locale)
      .formatToParts(-1)
      .find((part) => part.type === "minusSign")!.value
    : "";
  const localizedFraction = Array.from(fraction, (digit) => digitFormatter.format(Number(digit))).join("");

  return `${minusSign}${integerFormatter.format(whole)}${decimalSeparator}${localizedFraction}`;
}
