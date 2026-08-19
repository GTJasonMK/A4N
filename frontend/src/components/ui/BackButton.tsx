import React from "react";
import { ArrowLeft } from "lucide-react";
import { useLocation, useNavigate } from "react-router-dom";

/**
 * 智能返回 hook。
 *
 *   - 如果用户来自项目内导航（history.key !== "default"），调用 navigate(-1) 回上一页
 *   - 如果是直接进入（刷新 / 外部链接），跳转到默认目标
 *
 * 返回点击处理函数，由按钮或页面自定义流程复用。
 */
export const useSmartBack = (defaultTarget: string = "/") => {
  const navigate = useNavigate();
  const location = useLocation();
  return () => {
    if (location.key !== "default") {
      navigate(-1);
    } else {
      navigate(defaultTarget);
    }
  };
};

interface BackButtonProps {
  /** 显示文本，默认"返回" */
  label?: string;
  /** 仅显示箭头不显示文本 */
  iconOnly?: boolean;
  /** 直接进入时的默认目标，默认 "/" */
  defaultTarget?: string;
  /** 自定义点击行为（如需在返回前做清理） */
  onClick?: () => void;
  /** 按钮尺寸 */
  size?: "sm" | "md";
  title?: string;
  className?: string;
}

/**
 * 返回按钮：默认 navigate(-1)；如直接进入页面（无 history），跳转到默认目标。
 */
export const BackButton: React.FC<BackButtonProps> = ({
  label = "返回",
  iconOnly = false,
  defaultTarget = "/",
  onClick,
  size = "sm",
  title,
  className = "",
}) => {
  const smartBack = useSmartBack(defaultTarget);
  const handleClick = () => {
    if (onClick) onClick();
    else smartBack();
  };

  if (iconOnly) {
    return (
      <button
        type="button"
        onClick={handleClick}
        title={title || "返回上一页"}
        className={`a4n-icon-button pyqt-btn-icon ${
          size === "sm" ? "h-9 w-9 sm:h-7 sm:w-7" : ""
        } ${className}`}
      >
        <ArrowLeft size={size === "sm" ? 13 : 14} />
      </button>
    );
  }

  return (
    <button
      type="button"
      onClick={handleClick}
      title={title || "返回上一页"}
      className={`pyqt-btn-secondary ${
        size === "sm"
          ? "!min-h-9 !px-2 sm:!min-h-[26px]"
          : "!min-h-9 !px-3 sm:!min-h-[30px]"
      } text-xs inline-flex items-center gap-1 ${className}`}
    >
      <ArrowLeft size={11} />
      {label}
    </button>
  );
};
