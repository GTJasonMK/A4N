/**
 * 统一前端日志工具
 *
 * 设计要点：
 * - 任何级别都同步写入浏览器 console（开发者工具可见）
 * - info/warn/error 通过 Tauri invoke 转发到后端 tracing（target = `a4n::frontend`），
 *   写入与后端相同的 `a4n.log.YYYY-MM-DD` 文件，统一查看
 * - 接受 Error 对象时自动展开 name/message/stack，避免"[object Object]"
 * - 提供"循环防护"令牌：`logger.error` 内部不会再触发 console hook 的二次转发
 * - 提供全局错误 hook：window.error / unhandledrejection / console.error / console.warn
 */

import { invoke } from "@tauri-apps/api/core";

type Level = "debug" | "info" | "warn" | "error";

interface LogFields {
  [key: string]: unknown;
}

/** 是否在 Tauri 桌面运行环境 */
const isTauri = (): boolean => {
  if (typeof window === "undefined") return false;
  return Boolean((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__);
};

/**
 * 循环防护：当 logger 自己调 console.* 时，被 console hook 捕获后会再 forward，
 * 形成 logger → console → logger → … 死循环。
 * 用 try/finally 包裹的 token 标记 "正在通过 logger 路径输出"，console hook 见到就跳过。
 */
let _inLoggerCall = false;

/** 转发到后端 tracing */
async function forwardToBackend(
  level: Level,
  target: string,
  message: string,
  fields?: LogFields,
): Promise<void> {
  if (!isTauri()) return;
  if (level === "debug") return; // debug 仅前端 console
  try {
    await invoke("log_from_frontend", {
      input: {
        level,
        target,
        message,
        fields: fields ? sanitizeFields(fields) : null,
      },
    });
  } catch {
    // 日志机制本身失败必须静默，否则会冲掉真正的报错
  }
}

/** 把 fields 中的非 JSON 安全值（Error / Function / 循环引用）转成可序列化形式 */
function sanitizeFields(fields: LogFields): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(fields)) {
    out[k] = sanitizeValue(v);
  }
  return out;
}

function sanitizeValue(v: unknown): unknown {
  if (v instanceof Error) {
    const cause = (v as Error & { cause?: unknown }).cause;
    return {
      name: v.name,
      message: v.message,
      stack: v.stack,
      cause: cause !== undefined ? sanitizeValue(cause) : undefined,
    };
  }
  if (typeof v === "function") return `[function ${(v as Function).name || "anonymous"}]`;
  if (typeof v === "undefined") return null;
  if (v === null) return null;
  if (typeof v === "object") {
    try {
      JSON.stringify(v);
      return v;
    } catch {
      // 循环引用 / Symbol：退化为字符串
      return String(v);
    }
  }
  return v;
}

/** 保存浏览器原生 console 的引用，logger 内部用，避免 hook 后被再 forward */
const rawConsole = {
  debug: console.debug.bind(console),
  info: console.info.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
};

/** Logger 实例 —— 通过 `logger.scoped("MyComponent")` 创建带 target 的子 logger */
class Logger {
  constructor(private target: string = "frontend") {}

  scoped(target: string): Logger {
    return new Logger(target);
  }

  debug(message: string, fields?: LogFields): void {
    this._emit("debug", message, fields);
  }

  info(message: string, fields?: LogFields): void {
    this._emit("info", message, fields);
  }

  warn(message: string, fields?: LogFields): void {
    this._emit("warn", message, fields);
  }

  error(message: string, fields?: LogFields): void {
    this._emit("error", message, fields);
  }

  private _emit(level: Level, message: string, fields?: LogFields): void {
    _inLoggerCall = true;
    try {
      // 用原生 console，避免被 patch 后死循环
      const args: unknown[] = [`[${this.target}]`, message];
      if (fields && Object.keys(fields).length > 0) args.push(fields);
      rawConsole[level](...args);
    } finally {
      _inLoggerCall = false;
    }
    void forwardToBackend(level, this.target, message, fields);
  }
}

export const logger = new Logger();

/**
 * 安装全局错误捕获 + console.error/warn 劫持
 *
 * 必须在 main.tsx 最早期调用一次。
 *
 * 捕获范围：
 * 1. window.error          → 同步异常（包括异步抛出的 Error）
 * 2. unhandledrejection    → 未处理的 Promise 拒绝
 * 3. console.error/warn    → 任意第三方库或代码直接调用 console.* 的错误
 */
export function installGlobalErrorHandlers(): void {
  if (typeof window === "undefined") return;

  // 1) window.error
  window.addEventListener("error", (event) => {
    logger.error("全局未捕获异常", {
      message: event.message,
      filename: event.filename,
      line: event.lineno,
      column: event.colno,
      error: sanitizeValue(event.error),
    });
  });

  // 2) unhandledrejection
  window.addEventListener("unhandledrejection", (event) => {
    logger.error("Promise 未捕获拒绝", {
      reason: sanitizeValue(event.reason),
    });
  });

  // 3) 劫持 console.error / console.warn（带循环防护）
  const interceptedTarget = "console";
  console.error = (...args: unknown[]) => {
    rawConsole.error(...args);
    if (_inLoggerCall) return; // 防止 logger.error → console.error → forward 循环
    void forwardToBackend("error", interceptedTarget, summarizeArgs(args), {
      args: args.map(sanitizeValue),
    });
  };
  console.warn = (...args: unknown[]) => {
    rawConsole.warn(...args);
    if (_inLoggerCall) return;
    void forwardToBackend("warn", interceptedTarget, summarizeArgs(args), {
      args: args.map(sanitizeValue),
    });
  };
}

/** 把 console 多参数 args 摘要成单行 message */
function summarizeArgs(args: unknown[]): string {
  return args
    .map((a) => {
      if (a instanceof Error) return a.message;
      if (typeof a === "string") return a;
      try {
        return JSON.stringify(a);
      } catch {
        return String(a);
      }
    })
    .join(" ");
}
