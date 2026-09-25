import React from "react";
import ReactDOM from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import App from "./App";
import { I18nProvider, resolveInitialLocale } from "./i18n";
import { installGlobalErrorHandlers, logger } from "./utils/logger";
import { initializePortableStorage } from "./utils/portableStorage";
import "./index.css";
import { routerBasename } from "./utils/routerBase";

async function bootstrap() {
  // 先装全局错误捕获，确保后续任何阶段的异常都能被记录
  installGlobalErrorHandlers();
  logger.info("前端界面启动");

  await initializePortableStorage();
  document.documentElement.lang = resolveInitialLocale();
  const router = createBrowserRouter(
    [{ path: "*", element: <App /> }],
    { basename: routerBasename() },
  );

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <I18nProvider>
        <RouterProvider router={router} />
      </I18nProvider>
    </React.StrictMode>,
  );
}

void bootstrap();
