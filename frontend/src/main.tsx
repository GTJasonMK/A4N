import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import { THEME_MODE_STORAGE_KEY } from "./theme/activeTheme";
import { installGlobalErrorHandlers, logger } from "./utils/logger";
import {
  flushPortableStorage,
  initializePortableStorage,
  readPortableItem,
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
  window.addEventListener("beforeunload", () => {
    void flushPortableStorage();
  });

  const savedTheme = readPortableItem(THEME_MODE_STORAGE_KEY);
  if (savedTheme === "dark") {
    document.documentElement.classList.add("dark");
  } else {
    document.documentElement.classList.remove("dark");
  }

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <BrowserRouter basename={routerBasename()}>
        <App />
      </BrowserRouter>
    </React.StrictMode>,
  );
}

void bootstrap();
