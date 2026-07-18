import type { ReactNode } from "react";
import type { ModelRequestPreview } from "../../../api/requestPreview";

export interface ConfigItem<C> {
  id: number;
  isActive: boolean;
  config: C;
  readonly?: boolean;
  readonlyReason?: string;
  deleteDisabled?: boolean;
  deleteDisabledReason?: string;
}

export interface ConfigItemMeta {
  primary: string;
  secondary?: string;
  tertiary?: string;
}

export interface ConfigTestResult {
  status: "success" | "error";
  message: string;
  testedAt: string;
}

export interface ConfigEditorRenderContext {
  selectedId: number | null;
  readonly: boolean;
}

export interface ConfigListPanelProps<C> {
  title: string;
  items: ConfigItem<C>[];
  loading: boolean;
  onRefresh: () => Promise<void>;
  onCreate: (config: C) => Promise<void>;
  onUpdate: (id: number, config: C) => Promise<void>;
  onDelete: (id: number) => Promise<void>;
  onActivate: (id: number) => Promise<void>;
  onTest?: (id: number) => Promise<void>;
  onPreviewTest?: (id: number) => Promise<ModelRequestPreview>;
  formatItem: (config: C, isActive: boolean) => ConfigItemMeta;
  renderEditor: (
    form: C,
    setForm: (config: C) => void,
    context: ConfigEditorRenderContext,
  ) => ReactNode;
  defaultForm: C;
  validate?: (config: C) => string | null;
  activeLabel?: string;
  activateText?: string;
  activateSuccessMessage?: string;
  activateErrorMessage?: string;
}
