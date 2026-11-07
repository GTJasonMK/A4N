const baseUrl = process.env.A4N_BROWSER_E2E_BASE_URL;
if (!baseUrl) throw new Error("缺少 A4N_BROWSER_E2E_BASE_URL");

const apiBaseUrl = new URL("/api", baseUrl).toString().replace(/\/$/, "");

async function requestBrowserApi(path, args, { authenticated = true, method = "POST" } = {}) {
  const result = await browser.executeAsync(
    async (base, requestPath, requestArgs, requiresAuth, requestMethod, done) => {
      try {
        const headers = { "Content-Type": "application/json" };
        if (requiresAuth) {
          let token = window.localStorage.getItem("a4n:current_access_token");
          if (!token && window.__TAURI_INTERNALS__?.metadata) {
            const entries = await window.__TAURI_INTERNALS__.invoke("get_all_frontend_state");
            token = entries["a4n:current_access_token"] ?? null;
          }
          if (!token) throw new Error("当前浏览器没有已登录的 access token");
          headers.Authorization = `Bearer ${token}`;
        }

        const response = await fetch(
          `${base}/${requestPath}`,
          {
            method: requestMethod,
            headers,
            body: requestMethod === "GET" ? undefined : JSON.stringify(requestArgs),
          },
        );
        const body = response.status === 204 ? null : await response.json();
        done({ status: response.status, body });
      } catch (error) {
        done({ error: String(error) });
      }
    },
    apiBaseUrl,
    path,
    args,
    authenticated,
    method,
  );

  if (result.error) throw new Error(`${path} 请求失败：${result.error}`);
  return result;
}

export async function requestBrowserApiCommand(command, args = {}, options) {
  return requestBrowserApi(`commands/${encodeURIComponent(command)}`, args, options);
}

export async function callBrowserApiCommand(command, args = {}, options) {
  const response = await requestBrowserApiCommand(command, args, options);
  if (response.status < 200 || response.status >= 300) {
    throw new Error(`${command} 返回 HTTP ${response.status}：${JSON.stringify(response.body)}`);
  }
  return response.body;
}

export async function waitForBrowserGenerationTask(handle) {
  let summary;
  await browser.waitUntil(async () => {
    const response = await requestBrowserApi(
      `generation-tasks/${encodeURIComponent(handle.task_id)}?attempt=${handle.attempt}`,
      null,
      { method: "GET" },
    );
    if (response.status !== 200) throw new Error(`读取生成任务返回 HTTP ${response.status}：${JSON.stringify(response.body)}`);
    summary = response.body;
    return ["completed", "failed", "cancelled", "interrupted"].includes(summary.handle.status);
  }, { timeout: 120_000, interval: 250, timeoutMsg: "生成任务未进入终态" });
  return summary;
}

export async function persistBrowserAuth(auth) {
  await browser.execute(({ user, tokens }) => {
    window.localStorage.setItem("a4n:current_user_id", String(user.id));
    window.localStorage.setItem("a4n:current_session_id", tokens.session_id);
    window.localStorage.setItem("a4n:current_access_token", tokens.access_token);
    window.localStorage.setItem("a4n:current_refresh_token", tokens.refresh_token);
  }, auth);
}
