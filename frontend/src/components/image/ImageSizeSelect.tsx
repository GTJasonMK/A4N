import { ThemedSelect } from "../ui/ThemedSelect";
import { useEffect, useRef, useState } from "react";
import {
  listImageConfigs,
  normalizeImageSizes,
  subscribeImageGenerationConfigChanges,
  type ImageSize,
} from "../../api/image";
import { useI18n } from "../../i18n";

export interface ImageGenerationSizeState {
  error: Error | null;
  loading: boolean;
  size: ImageSize | null;
  sizes: ImageSize[];
  setSize: (size: ImageSize) => void;
}

export async function loadDefaultImageGenerationSize(current: ImageSize | null) {
  const configs = await listImageConfigs();
  const config = configs.find((item) => item.is_default);
  if (!config) throw new Error("未找到当前图片配置");
  const sizes = normalizeImageSizes(config.supported_sizes, `图片配置 ${config.name}`);
  const size = current && sizes.some((candidate) =>
    candidate.width === current.width && candidate.height === current.height,
  ) ? current : sizes[0];
  return { sizes, size };
}

export function useImageGenerationSize(): ImageGenerationSizeState {
  const [sizes, setSizes] = useState<ImageSize[]>([]);
  const [size, setSelectedSize] = useState<ImageSize | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [loading, setLoading] = useState(true);
  const sizeRef = useRef<ImageSize | null>(null);
  const setSize = (next: ImageSize) => {
    sizeRef.current = next;
    setSelectedSize(next);
  };

  useEffect(() => {
    let active = true;
    let requestSeq = 0;
    const load = async () => {
      const request = ++requestSeq;
      setLoading(true);
      try {
        const result = await loadDefaultImageGenerationSize(sizeRef.current);
        if (!active || request !== requestSeq) return;
        setSizes(result.sizes);
        sizeRef.current = result.size;
        setSelectedSize(result.size);
        setError(null);
      } catch (reason: unknown) {
        if (!active || request !== requestSeq) return;
        setSizes([]);
        sizeRef.current = null;
        setSelectedSize(null);
        setError(reason instanceof Error ? reason : new Error(String(reason)));
      } finally {
        if (active && request === requestSeq) setLoading(false);
      }
    };
    const refresh = (event: CustomEvent<Error | null>) => {
      if (event.detail) {
        requestSeq += 1;
        setLoading(false);
        setSizes([]);
        sizeRef.current = null;
        setSelectedSize(null);
        setError(event.detail);
        return;
      }
      void load();
    };
    void load();
    const unsubscribe = subscribeImageGenerationConfigChanges(refresh);
    return () => {
      active = false;
      requestSeq += 1;
      unsubscribe();
    };
  }, []);

  return { error, loading, size, sizes, setSize };
}

interface ImageSizeSelectProps {
  disabled?: boolean;
  error?: Error | null;
  label?: string;
  loading?: boolean;
  onChange: (size: ImageSize) => void;
  sizes: readonly ImageSize[];
  value: ImageSize | null;
}

export function ImageSizeSelect({
  disabled = false,
  error = null,
  label,
  loading = false,
  onChange,
  sizes,
  value,
}: ImageSizeSelectProps) {
  const { t } = useI18n();
  const selectedValue = value ? `${value.width}x${value.height}` : "";
  return (
    <label className="block min-w-0 text-caption text-book-text-muted">
      {label ?? t("settingsForm.supportedSizes")}
      <ThemedSelect
        value={selectedValue}
        disabled={disabled || loading || error != null || sizes.length === 0}
        onChange={(selectedValue) => {
          const next = sizes.find(
            (candidate) => `${candidate.width}x${candidate.height}` === selectedValue,
          );
          if (next) onChange(next);
        }}
        className="pyqt-input mt-1 w-full min-w-0 text-xs"
      >
        <option value="" disabled>
          {loading
            ? t("settingsForm.imageSizeLoading")
            : error || sizes.length === 0
              ? t("settingsForm.imageSizeUnavailable")
              : t("settingsForm.selectImageSize")}
        </option>
        {sizes.map((candidate) => (
          <option
            key={`${candidate.width}x${candidate.height}`}
            value={`${candidate.width}x${candidate.height}`}
          >
            {candidate.width} x {candidate.height}
          </option>
        ))}
      </ThemedSelect>
      {error ? (
        <span role="alert" className="mt-1 block text-small text-warning">
          {t("settingsForm.imageSizeUnavailable")}
        </span>
      ) : null}
    </label>
  );
}
