import React from "react";

interface PageHeaderProps {
  leading?: React.ReactNode;
  icon?: React.ReactNode;
  eyebrow?: React.ReactNode;
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  meta?: React.ReactNode;
  actions?: React.ReactNode;
  titleId?: string;
  className?: string;
}

/**
 * 应用级页头。
 *
 * 只统一信息结构与操作落位，不限制各业务页能展示的能力：
 * 左侧返回/入口，中间身份与状态，右侧页面专属操作。
 */
export const PageHeader: React.FC<PageHeaderProps> = ({
  leading,
  icon,
  eyebrow,
  title,
  subtitle,
  meta,
  actions,
  titleId,
  className = "",
}) => (
  <header className={`a4n-page-header a4n-app-page-header ${className}`.trim()}>
    {leading != null ? <div className="a4n-app-page-header__leading">{leading}</div> : null}
    {icon != null ? <div className="a4n-app-page-header__icon">{icon}</div> : null}

    <div className="a4n-app-page-header__identity">
      {eyebrow != null ? <div className="a4n-kicker">{eyebrow}</div> : null}
      <div className="a4n-app-page-header__title-row">
        <h1 id={titleId} className="a4n-app-page-header__title">
          {title}
        </h1>
        {meta != null ? <div className="a4n-app-page-header__meta">{meta}</div> : null}
      </div>
      {subtitle != null ? <div className="a4n-app-page-header__subtitle">{subtitle}</div> : null}
    </div>

    {actions != null ? <div className="a4n-app-page-header__actions">{actions}</div> : null}
  </header>
);

export default PageHeader;
