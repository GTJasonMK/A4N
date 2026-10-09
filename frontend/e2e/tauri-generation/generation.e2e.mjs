const composerSelector = ".chat-composer textarea";

const pageContains = async (text) => browser.execute(
  (value) => document.body.innerText.includes(value),
  text,
);

const waitForPageText = async (text) => {
  try {
    await browser.waitUntil(() => pageContains(text));
  } catch (error) {
    const body = await browser.execute(() => document.body.innerText);
    throw new Error(`${error.message}; waiting for ${text}; body:\n${body}`);
  }
};

const elementExists = async (selector) => browser.execute(
  (value) => Boolean(document.querySelector(value)),
  selector,
);

const fill = async (selector, value) => browser.execute((target, nextValue) => {
  const element = document.querySelector(target);
  if (!element) throw new Error(`missing element: ${target}`);
  const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(element), "value")?.set;
  setter?.call(element, nextValue);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}, selector, value);

const click = async (selector) => browser.execute((target) => {
  const element = document.querySelector(target);
  if (!element) throw new Error(`missing element: ${target}`);
  element.click();
}, selector);

const clickButton = async (label) => browser.execute((text) => {
  const button = [...document.querySelectorAll("button")]
    .find((candidate) => candidate.textContent?.trim() === text);
  if (!button) throw new Error(`missing button: ${text}`);
  button.click();
}, label);

const createProject = async () => {
  await browser.$("button*=创建项目").click();
  await browser.waitUntil(() => pageContains("AI 辅助创作"));
  await browser.$("button*=AI 辅助创作").click();
  await browser.waitUntil(async () => /\/inspiration\/\d+$/.test(new URL(await browser.getUrl()).pathname));
  const match = (await browser.getUrl()).match(/\/inspiration\/(\d+)$/);
  if (!match) throw new Error("AI 辅助创作没有打开新灵感项目");
  return Number(match[1]);
};

const navigate = async (path) => {
  await browser.execute((nextPath) => {
    window.history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, path);
};

const observeFirstText = async (text, key) => {
  await browser.execute((target, resultKey) => {
    const record = () => {
      if (document.body?.innerText.includes(target) && window[resultKey] == null) {
        window[resultKey] = Date.now();
        return true;
      }
      return false;
    };
    window[resultKey] = null;
    if (record()) return;
    const observer = new MutationObserver(() => {
      if (record()) observer.disconnect();
    });
    observer.observe(document.documentElement, { childList: true, subtree: true, characterData: true });
  }, text, key);
};

const send = async (message) => {
  await browser.waitUntil(() => browser.execute((selector) => {
    const input = document.querySelector(selector);
    return Boolean(input && !input.disabled);
  }, composerSelector));
  await fill(composerSelector, message);
  await clickButton("发送");
  await browser.waitUntil(() => pageContains("发送中"));
};

describe("Tauri generation user flow", () => {
  it("creates a project and completes mock generation across route remount", async () => {
    const { username, password } = process.env;
    if (!username || !password) throw new Error("A4N_WDIO_USERNAME and A4N_WDIO_PASSWORD are required");
    const suffix = Date.now();
    const userPrompt = `A4N_TAURI_GENERATION_INPUT:${suffix}`;
    const remountReply = `A4N_TAURI_GENERATION_REPLY:${suffix}`;

    const usernameSelector = 'input[placeholder="输入用户名"]';
    await browser.waitUntil(() => elementExists(usernameSelector));
    await fill(usernameSelector, username);
    await fill('input[placeholder="输入密码"]', password);
    const loginButton = 'form button[type="submit"]';
    await browser.waitUntil(() => browser.execute((selector) => {
      const button = document.querySelector(selector);
      return Boolean(button && !button.disabled);
    }, loginButton));
    await click(loginButton);
    await browser.waitUntil(async () => !(await elementExists(usernameSelector)));

    const projectId = await createProject();
    const route = `/inspiration/${projectId}`;
    await navigate(route);
    await browser.waitUntil(() => elementExists(composerSelector));
    await observeFirstText(remountReply, "__a4nRemountFirstTextAt");
    await send(userPrompt);
    await waitForPageText(remountReply);
    const remountFirstTextAt = await browser.execute(() => window.__a4nRemountFirstTextAt);

    await navigate("/");
    await browser.waitUntil(async () => !(await elementExists(composerSelector)));
    await navigate(route);
    await browser.waitUntil(() => elementExists(composerSelector));
    await browser.waitUntil(async () => await pageContains(remountReply) && !(await pageContains("发送中")));
    const remountOccurrences = await browser.execute(
      (text) => document.body.innerText.split(text).length - 1,
      remountReply,
    );
    const remountCompleted = !(await pageContains("发送中"));
    if (!remountCompleted || remountOccurrences !== 1) {
      throw new Error("Tauri 页面重挂载后没有唯一且完成的 mock 回复");
    }

    process.stdout.write(`A4N_TAURI_WRY_RESULT=${JSON.stringify({
      remountFirstTextAt,
      remountOccurrences,
      remountCompleted,
    })}\n`);
  });
});
