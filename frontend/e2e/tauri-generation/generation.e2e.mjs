const requiredEnv = (name) => {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
};

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

describe("real Tauri generation lifecycle", () => {
  it("consumes Channel, remounts by active cursor, and cleans up on Destroyed", async () => {
    const username = requiredEnv("A4N_WDIO_USERNAME");
    const password = requiredEnv("A4N_WDIO_PASSWORD");
    const projectId = requiredEnv("A4N_WDIO_PROJECT_ID");
    const suffix = requiredEnv("A4N_WDIO_SUFFIX");
    const route = `/inspiration/${projectId}`;
    const remountMessage = `tauri-wry-remount-${suffix}`;
    const destroyMessage = `tauri-wry-destroyed-${suffix}`;
    const remountFirstText = "真实 Wry 首字";
    const remountReply = "真实 Wry 首字重挂载完成";
    const destroyFirstText = "真实 Wry 销毁清理首字";

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

    await navigate(route);
    await browser.waitUntil(() => elementExists(composerSelector));
    await observeFirstText(remountFirstText, "__a4nRemountFirstTextAt");
    await send(remountMessage);
    await waitForPageText(remountFirstText);
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

    await observeFirstText(destroyFirstText, "__a4nDestroyFirstTextAt");
    await send(destroyMessage);
    await browser.waitUntil(() => pageContains(destroyFirstText));
    const destroyFirstTextAt = await browser.execute(() => window.__a4nDestroyFirstTextAt);

    process.stdout.write(`A4N_TAURI_WRY_RESULT=${JSON.stringify({
      remountFirstTextAt,
      destroyFirstTextAt,
      remountOccurrences,
      remountCompleted,
      destroyStillSending: await pageContains("发送中"),
    })}\n`);
    await browser.execute(() => {
      const { invoke, metadata } = window.__TAURI_INTERNALS__;
      // Let executeAsync reply before destroying its WebDriver session.
      setTimeout(
        () => void invoke("plugin:window|destroy", { label: metadata.currentWindow.label }),
        100,
      );
    });
    await new Promise((resolve) => setTimeout(resolve, 500));
  });
});
