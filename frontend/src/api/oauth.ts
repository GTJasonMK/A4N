import { oauth2AuthorizeUrl, oauth2Callback, persistAuthResult } from "./auth";
import { readCurrentSessionId } from "./authStorage";
import type { AuthResult, OAuth2BrowserInput } from "./generated/auth";
import { isTauriRuntime, runtimeInvoke } from "./runtime";
import { getActiveLocale, translateMessage } from "../i18n";
import { routerBasename } from "../utils/routerBase";

const STORAGE_KEY = "a4n.oauth.pending";
const AUTHORIZATION_TTL_MS = 10 * 60 * 1000;

interface PendingAuthorization {
  provider: string;
  state: string;
  codeVerifier: string;
  redirectUri: string;
  returnTo: string;
  createdAt: number;
}

interface CompletedAuthorization {
  auth: AuthResult;
  returnTo: string;
}

let completion: { url: string; promise: Promise<CompletedAuthorization> } | null = null;
let browserLogin: Promise<string> | null = null;
let authorizationVersion = 0;

const message = (key: "auth.oauthRequestMissing" | "auth.oauthStateMismatch" | "auth.oauthCancelled" | "auth.oauthCallbackInvalid") =>
  translateMessage(getActiveLocale(), key);

const base64Url = (bytes: Uint8Array): string =>
  btoa(String.fromCharCode(...bytes)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");

export async function beginOAuthLogin(provider: string): Promise<string | null> {
  const version = ++authorizationVersion;
  const state = base64Url(crypto.getRandomValues(new Uint8Array(32)));
  const codeVerifier = base64Url(crypto.getRandomValues(new Uint8Array(32)));
  const challenge = base64Url(new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(codeVerifier))));
  const desktop = isTauriRuntime();
  const basename = routerBasename() ?? "";
  const requestedRedirect = desktop ? null : new URL(`${basename}/auth/callback`, window.location.origin).href;
  const authorizationUrl = await oauth2AuthorizeUrl({
    provider,
    redirect_uri: requestedRedirect,
    state,
    code_challenge: challenge,
  });
  if (version !== authorizationVersion) throw new DOMException("OAuth cancelled", "AbortError");
  const redirectUri = new URL(authorizationUrl).searchParams.get("redirect_uri");
  if (!redirectUri || (requestedRedirect !== null && redirectUri !== requestedRedirect)) {
    throw new Error(message("auth.oauthCallbackInvalid"));
  }
  const pending: PendingAuthorization = {
    provider, state, codeVerifier, redirectUri,
    returnTo: (window.location.pathname.slice(basename.length) || "/") + window.location.search + window.location.hash,
    createdAt: Date.now(),
  };
  sessionStorage.setItem(STORAGE_KEY, JSON.stringify(pending));
  completion = null;
  if (!desktop) {
    window.location.assign(authorizationUrl);
    return null;
  }
  const input: OAuth2BrowserInput = { authorization_url: authorizationUrl, redirect_uri: redirectUri };
  browserLogin = runtimeInvoke<string>("oauth2_browser_login", { input });
  try {
    return await browserLogin;
  } catch (error) {
    sessionStorage.removeItem(STORAGE_KEY);
    throw error;
  } finally {
    browserLogin = null;
  }
}

export async function cancelOAuthLogin(): Promise<void> {
  authorizationVersion += 1;
  sessionStorage.removeItem(STORAGE_KEY);
  if (browserLogin) {
    const current = browserLogin;
    await runtimeInvoke<void>("cancel_oauth2_browser_login");
    await current.catch(() => undefined);
  }
}

export function completeOAuthLogin(callbackUrl: string): Promise<CompletedAuthorization> {
  if (completion?.url === callbackUrl) return completion.promise;
  const promise = exchangeAuthorization(callbackUrl);
  completion = { url: callbackUrl, promise };
  return promise;
}

async function exchangeAuthorization(callbackUrl: string): Promise<CompletedAuthorization> {
  const version = authorizationVersion;
  const sessionId = readCurrentSessionId();
  const stored = sessionStorage.getItem(STORAGE_KEY);
  if (!stored) throw new Error(message("auth.oauthRequestMissing"));
  let pending: unknown;
  try {
    pending = JSON.parse(stored);
  } catch {
    sessionStorage.removeItem(STORAGE_KEY);
    throw new Error(message("auth.oauthRequestMissing"));
  }
  if (!isPendingAuthorization(pending) || Date.now() - pending.createdAt > AUTHORIZATION_TTL_MS || pending.createdAt > Date.now()) {
    sessionStorage.removeItem(STORAGE_KEY);
    throw new Error(message("auth.oauthRequestMissing"));
  }
  const callback = new URL(callbackUrl);
  const expected = new URL(pending.redirectUri);
  if (callback.origin !== expected.origin || callback.pathname !== expected.pathname || callback.hash) {
    throw new Error(message("auth.oauthCallbackInvalid"));
  }
  if (callback.searchParams.getAll("state").length !== 1 || callback.searchParams.get("state") !== pending.state) {
    throw new Error(message("auth.oauthStateMismatch"));
  }
  sessionStorage.removeItem(STORAGE_KEY);
  const error = callback.searchParams.get("error");
  if (error) {
    throw new Error(error === "access_denied" ? message("auth.oauthCancelled") : `${message("auth.oauthCallbackInvalid")}: ${error}`);
  }
  const code = callback.searchParams.get("code");
  if (callback.searchParams.getAll("code").length !== 1 || !code?.trim()) {
    throw new Error(message("auth.oauthCallbackInvalid"));
  }
  const auth = await oauth2Callback({
    provider: pending.provider,
    code,
    redirect_uri: pending.redirectUri,
    code_verifier: pending.codeVerifier,
  });
  if (version !== authorizationVersion || readCurrentSessionId() !== sessionId) {
    throw new Error(translateMessage(getActiveLocale(), "auth.sessionChanged"));
  }
  await persistAuthResult(auth);
  return { auth, returnTo: pending.returnTo };
}

function isPendingAuthorization(value: unknown): value is PendingAuthorization {
  if (!value || typeof value !== "object") return false;
  const item = value as Record<string, unknown>;
  return ["provider", "state", "codeVerifier", "redirectUri", "returnTo"].every(
    (key) => typeof item[key] === "string" && (item[key] as string).length > 0,
  ) && typeof item.createdAt === "number" && Number.isFinite(item.createdAt)
    && (item.returnTo as string).startsWith("/") && !(item.returnTo as string).startsWith("//");
}
