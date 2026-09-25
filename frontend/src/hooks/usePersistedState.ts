import { useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { readPortableItem, writePortableItem } from "../utils/portableStorage";
import { extractErrorMessage } from "../api";
import { useToast } from "../components/feedback/Toast";
import { getActiveLocale, translateMessage } from "../i18n";

type UsePersistedStateOptions<T> = {
  parse?: (raw: string) => T;
  serialize?: (value: T) => string;
};

const defaultParse = <T>(raw: string): T => JSON.parse(raw) as T;
const defaultSerialize = <T>(value: T): string => JSON.stringify(value);

const readPersisted = <T>(
  key: string | null | undefined,
  defaultValue: T,
  parse: (raw: string) => T,
): T => {
  if (!key) return defaultValue;
  const raw = readPortableItem(key);
  if (raw === null) return defaultValue;
  return parse(raw);
};

export const usePersistedState = <T>(
  storageKey: string | null | undefined,
  initialValue: T,
  opts: UsePersistedStateOptions<T> = {},
): [T, Dispatch<SetStateAction<T>>] => {
  const parse = opts.parse ?? defaultParse<T>;
  const serialize = opts.serialize ?? defaultSerialize<T>;
  const parseRef = useRef(parse);
  const serializeRef = useRef(serialize);
  const skipNextWriteKeyRef = useRef<string | null | undefined>(null);

  parseRef.current = parse;
  serializeRef.current = serialize;

  const [value, setValue] = useState<T>(() =>
    readPersisted(storageKey, initialValue, parse),
  );

  useEffect(() => {
    // 只在 key 或初始值语义变化时重新从存储读取，避免调用方传入内联 parse/serialize
    // 时触发 “每次渲染都重置状态” 的读写乒乓。
    skipNextWriteKeyRef.current = storageKey;
    setValue(readPersisted(storageKey, initialValue, parseRef.current));
  }, [initialValue, storageKey]);

  useEffect(() => {
    if (!storageKey) return;
    if (skipNextWriteKeyRef.current === storageKey) {
      skipNextWriteKeyRef.current = null;
      return;
    }
    void writePortableItem(storageKey, serializeRef.current(value)).catch((error) => {
      useToast.getState().addToast(
        extractErrorMessage(error, translateMessage(getActiveLocale(), "errors.saveFailed")),
        "error",
      );
    });
  }, [storageKey, value]);

  return [value, setValue];
};
