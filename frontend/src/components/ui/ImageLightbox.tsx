import React, {
  createContext,
  forwardRef,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ImgHTMLAttributes,
  type ReactNode,
} from "react";
import { Minus, Plus, RotateCcw, ZoomIn } from "lucide-react";
import { useI18n } from "../../i18n";
import { Modal } from "./Modal";
import { BookIconButton } from "./BookButton";

type ImageLightboxTarget = {
  src: string;
  alt: string;
};

const ImageLightboxContext = createContext<{
  openImage: (target: ImageLightboxTarget) => void;
} | null>(null);
const MIN_SCALE = 1;
const MAX_SCALE = 4;
const SCALE_STEP = 0.25;

const clampScale = (value: number): number =>
  Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));

const formatScale = (value: number): string => `${Math.round(value * 100)}%`;

export function ImageZoomButton({ src, alt = "", className = "" }: ImageLightboxTarget & { className?: string }) {
  const context = useContext(ImageLightboxContext);
  if (!context) throw new Error("ImageZoomButton must be used inside ImageLightboxProvider");
  const { t } = useI18n();
  return (
    <BookIconButton
      label={t("common.imagePreview.title")}
      size="sm"
      variant="secondary"
      className={className}
      onClick={(event) => { event.stopPropagation(); context.openImage({ src, alt }); }}
    >
      <ZoomIn size={14} />
    </BookIconButton>
  );
}

export const ZoomableImage = forwardRef<HTMLImageElement, ImgHTMLAttributes<HTMLImageElement>>(
  function ZoomableImage({ alt = "", className = "", onClick, onKeyDown, tabIndex = 0, ...props }, ref) {
    const context = useContext(ImageLightboxContext);
    if (!context) throw new Error("ZoomableImage must be used inside ImageLightboxProvider");
    const { t } = useI18n();
    const src = typeof props.src === "string" ? props.src : "";
    const hidden = props["aria-hidden"] === true || props["aria-hidden"] === "true";
    const keyboardAccessible = Boolean(src) && tabIndex >= 0 && !hidden;
    const accessibleLabel = props["aria-label"] || alt || t("common.imagePreview.title");
    const handleClick = (event: React.MouseEvent<HTMLImageElement>) => {
      onClick?.(event);
      if (event.defaultPrevented || !src || hidden) return;
      event.stopPropagation();
      context.openImage({ src, alt });
    };

    const handleKeyDown = (event: React.KeyboardEvent<HTMLImageElement>) => {
      onKeyDown?.(event);
      if (event.defaultPrevented || !keyboardAccessible) return;
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      if (src) context.openImage({ src, alt });
    };

    return (
      <img
        {...props}
        ref={ref}
        alt={alt}
        role={keyboardAccessible ? "button" : undefined}
        tabIndex={keyboardAccessible ? tabIndex : undefined}
        aria-label={keyboardAccessible ? accessibleLabel || undefined : props["aria-label"]}
        className={`${src && !hidden ? "cursor-zoom-in" : ""} ${className}`.trim()}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
      />
    );
  },
);

const ImageLightboxDialog: React.FC<{
  target: ImageLightboxTarget | null;
  onClose: () => void;
}> = ({ target, onClose }) => {
  const { t } = useI18n();
  const [scale, setScale] = useState(MIN_SCALE);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    originX: number;
    originY: number;
  } | null>(null);

  useEffect(() => {
    setScale(MIN_SCALE);
    setOffset({ x: 0, y: 0 });
    dragRef.current = null;
    setDragging(false);
  }, [target?.src]);

  const setZoom = useCallback((nextScale: number) => {
    const next = clampScale(nextScale);
    setScale(next);
    if (next === MIN_SCALE) setOffset({ x: 0, y: 0 });
  }, []);

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    event.preventDefault();
    setZoom(scale + (event.deltaY < 0 ? SCALE_STEP : -SCALE_STEP));
  };

  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (scale <= MIN_SCALE) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragging(true);
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      originX: offset.x,
      originY: offset.y,
    };
  };

  const handlePointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    setOffset({
      x: drag.originX + event.clientX - drag.startX,
      y: drag.originY + event.clientY - drag.startY,
    });
  };

  const stopDragging = (event: React.PointerEvent<HTMLDivElement>) => {
    if (dragRef.current?.pointerId !== event.pointerId) return;
    dragRef.current = null;
    setDragging(false);
  };

  return (
    <Modal
      isOpen={target != null}
      onClose={onClose}
      title={target?.alt || t("common.imagePreview.title")}
      subtitle={t("common.imagePreview.hint")}
      footer={
        <div className="a4n-image-lightbox-toolbar">
          <span className="text-small font-medium text-book-text-muted">{formatScale(scale)}</span>
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              className="a4n-icon-button inline-flex h-10 w-10 items-center justify-center rounded-xl"
              onClick={() => setZoom(scale - SCALE_STEP)}
              disabled={scale <= MIN_SCALE}
              aria-label={t("common.imagePreview.zoomOut")}
              title={t("common.imagePreview.zoomOut")}
            >
              <Minus size={16} />
            </button>
            <button
              type="button"
              className="a4n-icon-button inline-flex h-10 w-10 items-center justify-center rounded-xl"
              onClick={() => setZoom(MIN_SCALE)}
              disabled={scale === MIN_SCALE && offset.x === 0 && offset.y === 0}
              aria-label={t("common.imagePreview.reset")}
              title={t("common.imagePreview.reset")}
            >
              <RotateCcw size={15} />
            </button>
            <button
              type="button"
              className="a4n-icon-button inline-flex h-10 w-10 items-center justify-center rounded-xl"
              onClick={() => setZoom(scale + SCALE_STEP)}
              disabled={scale >= MAX_SCALE}
              aria-label={t("common.imagePreview.zoomIn")}
              title={t("common.imagePreview.zoomIn")}
            >
              <Plus size={16} />
            </button>
          </div>
        </div>
      }
      maxWidthClassName="max-w-[min(96vw,1400px)]"
      className="a4n-image-lightbox-panel"
      bodyClassName="a4n-image-lightbox-body"
      zIndexClassName="z-[100]"
    >
      {target ? (
        <div
          className={`a4n-image-lightbox-stage${dragging ? " is-dragging" : ""}`}
          onWheel={handleWheel}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={stopDragging}
          onPointerCancel={stopDragging}
          onDoubleClick={() => setZoom(scale === MIN_SCALE ? 2 : MIN_SCALE)}
          title={t("common.imagePreview.hint")}
        >
          <img
            src={target.src}
            alt={target.alt}
            draggable={false}
            className="a4n-image-lightbox-image"
            style={{ transform: `translate3d(${offset.x}px, ${offset.y}px, 0) scale(${scale})` }}
          />
        </div>
      ) : null}
    </Modal>
  );
};

export const ImageLightboxProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [target, setTarget] = useState<ImageLightboxTarget | null>(null);
  const openImage = useCallback((nextTarget: ImageLightboxTarget) => {
    setTarget(nextTarget);
  }, []);
  const closeImage = useCallback(() => setTarget(null), []);

  return (
    <ImageLightboxContext.Provider value={{ openImage }}>
      {children}
      <ImageLightboxDialog target={target} onClose={closeImage} />
    </ImageLightboxContext.Provider>
  );
};
