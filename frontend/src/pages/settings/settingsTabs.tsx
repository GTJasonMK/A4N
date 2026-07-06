import React from "react";
import {
  Activity,
  Bot,
  Brain,
  FileText,
  Film,
  Image as ImageIcon,
  Info,
  Palette,
  ScrollText,
  Shield,
  Wrench,
} from "lucide-react";

export type TabId =
  | "llm"
  | "embedding"
  | "image"
  | "video"
  | "queue"
  | "prompt"
  | "theme"
  | "advanced"
  | "admin"
  | "logs"
  | "about";

interface TabDef {
  id: TabId;
  label: string;
  icon: React.ReactNode;
  scopeLabel: string;
  groupLabel: "本机设置" | "云端管理" | "服务器诊断";
}

export const TABS: TabDef[] = [
  { id: "llm", label: "LLM 服务", icon: <Bot size={14} />, scopeLabel: "配置 / LLM", groupLabel: "本机设置" },
  { id: "embedding", label: "嵌入模型", icon: <Brain size={14} />, scopeLabel: "配置 / 嵌入", groupLabel: "本机设置" },
  { id: "image", label: "生图模型", icon: <ImageIcon size={14} />, scopeLabel: "配置 / 图像", groupLabel: "本机设置" },
  { id: "video", label: "视频生成", icon: <Film size={14} />, scopeLabel: "配置 / 视频", groupLabel: "本机设置" },
  { id: "prompt", label: "提示词管理", icon: <FileText size={14} />, scopeLabel: "内容 / 提示词", groupLabel: "本机设置" },
  { id: "theme", label: "主题配置", icon: <Palette size={14} />, scopeLabel: "外观 / 主题", groupLabel: "本机设置" },
  { id: "advanced", label: "高级配置", icon: <Wrench size={14} />, scopeLabel: "系统 / 高级", groupLabel: "本机设置" },
  { id: "admin", label: "管理员", icon: <Shield size={14} />, scopeLabel: "系统 / 管理员", groupLabel: "云端管理" },
  { id: "queue", label: "请求队列", icon: <Activity size={14} />, scopeLabel: "运维 / 队列", groupLabel: "服务器诊断" },
  { id: "logs", label: "日志", icon: <ScrollText size={14} />, scopeLabel: "系统 / 日志", groupLabel: "服务器诊断" },
  { id: "about", label: "关于", icon: <Info size={14} />, scopeLabel: "系统 / 关于", groupLabel: "服务器诊断" },
];

export const TAB_IDS = TABS.map((tab) => tab.id) as readonly TabId[];
