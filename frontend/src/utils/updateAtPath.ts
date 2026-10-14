export type ValuePath = Array<string | number>;
export type UpdateValue = (path: ValuePath, value: unknown) => void;

export function updateAtPath<T>(
  source: T,
  path: ValuePath,
  value: unknown,
  label: string,
): T {
  const invalidPath = () => new Error(`Invalid ${label} edit path: ${path.join(".")}`);
  if (path.length === 0) throw invalidPath();

  const copyAt = (current: unknown, index: number): unknown => {
    if (current == null || typeof current !== "object") throw invalidPath();
    const key = path[index];
    if (key == null) throw invalidPath();
    const copy: object = Array.isArray(current)
      ? current.slice()
      : { ...current as Record<string, unknown> };
    if (index === path.length - 1) {
      if (!Reflect.set(copy, key, value)) throw invalidPath();
      return copy;
    }
    if (!Reflect.set(copy, key, copyAt(Reflect.get(current, key), index + 1))) {
      throw invalidPath();
    }
    return copy;
  };

  return copyAt(source, 0) as T;
}
