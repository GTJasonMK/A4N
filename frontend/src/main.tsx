import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { I18nProvider, resolveInitialLocale } from "./i18n";
import { installGlobalErrorHandlers, logger } from "./utils/logger";
import {
  flushPortableStorage,
  initializePortableStorage,
} from "./utils/portableStorage";
import "./index.css";

function routerBasename(): string | undefined {
  const value = import.meta.env.VITE_A4N_BASE_PATH?.trim();
  if (!value || value === "/") return undefined;
  return value.startsWith("/")
    ? value.replace(/\/$/, "")
    : `/${value.replace(/\/$/, "")}`;
}

async function bootstrap() {
  // 先装全局错误捕获，确保后续任何阶段的异常都能被记录
  installGlobalErrorHandlers();
  logger.info("前端界面启动");

  await initializePortableStorage();
  document.documentElement.lang = resolveInitialLocale();
  window.addEventListener("beforeunload", () => {
    void flushPortableStorage();
  });

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <BrowserRouter basename={routerBasename()}>
        <I18nProvider>
          <App />
        </I18nProvider>
      </BrowserRouter>
    </React.StrictMode>,
  );
}

void bootstrap();
