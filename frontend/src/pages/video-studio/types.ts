import type { ComponentType } from "react";
import type { Edge, Node } from "@xyflow/react";

export type StudioNodeData = {
  label: string;
  nodeKind: string;
  refType: string | null;
  refId: number | null;
  state: string;
  config: Record<string, unknown>;
  runStatus?: "running" | "completed" | "failed";
  runElapsedSeconds?: number | null;
};

export type StudioNode = Node<StudioNodeData>;
export type StudioEdge = Edge<{ role: string; config: Record<string, unknown> }>;
export type StudioTemplateIcon = ComponentType<{ size?: string | number; className?: string }>;
