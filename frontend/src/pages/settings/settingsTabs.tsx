import React from "react";
import {
  Activity,
  Bot,
  BookOpen,
  Brain,
  CircleUserRound,
  FileText,
  Film,
  Image as ImageIcon,
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
  | "guide"
  | "advanced"
  | "admin"
  | "logs"
  | "about";

interface TabDef {
  id: TabId;
  label: string;
  icon: React.ReactNode;
  groupLabel: "本机设置" | "云端管理" | "服务器诊断";
}

export const TABS: TabDef[] = [
  { id: "llm", label: "LLM 服务", icon: <Bot size={14} />, groupLabel: "本机设置" },
  { id: "embedding", label: "嵌入模型", icon: <Brain size={14} />, groupLabel: "本机设置" },
  { id: "image", label: "生图模型", icon: <ImageIcon size={14} />, groupLabel: "本机设置" },
  { id: "video", label: "视频生成", icon: <Film size={14} />, groupLabel: "本机设置" },
  { id: "prompt", label: "提示词管理", icon: <FileText size={14} />, groupLabel: "本机设置" },
  { id: "theme", label: "主题配置", icon: <Palette size={14} />, groupLabel: "本机设置" },
  { id: "guide", label: "使用指南", icon: <BookOpen size={14} />, groupLabel: "本机设置" },
  { id: "advanced", label: "高级配置", icon: <Wrench size={14} />, groupLabel: "本机设置" },
  { id: "admin", label: "管理员", icon: <Shield size={14} />, groupLabel: "云端管理" },
  { id: "queue", label: "请求队列", icon: <Activity size={14} />, groupLabel: "服务器诊断" },
  { id: "logs", label: "日志", icon: <ScrollText size={14} />, groupLabel: "服务器诊断" },
  { id: "about", label: "账户与应用", icon: <CircleUserRound size={14} />, groupLabel: "本机设置" },
];

export const TAB_IDS = TABS.map((tab) => tab.id) as readonly TabId[];
