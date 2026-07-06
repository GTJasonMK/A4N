import { Buffer } from "node:buffer";

export function env(name, fallback) {
  const value = process.env[name];
  return value && value.trim() ? value.trim() : fallback;
}

export function intEnv(name, fallback) {
  const raw = process.env[name];
  if (!raw || !raw.trim()) return fallback;
  const value = Number(raw);
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${name} must be a positive integer`);
  }
  return value;
}

export function numberOrNull(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

export async function fetchWithContext(url, options, label) {
  const fetchOptions = options ? { ...options } : undefined;
  const timeoutMs = numberOrNull(fetchOptions?.timeoutMs);
  if (fetchOptions) delete fetchOptions.timeoutMs;
  let timeoutId = null;
  if (timeoutMs && timeoutMs > 0 && !fetchOptions?.signal) {
    const controller = new AbortController();
    if (fetchOptions) {
      fetchOptions.signal = controller.signal;
    }
    timeoutId = setTimeout(() => controller.abort(), timeoutMs);
  }
  const method = String(options?.method ?? "GET").toUpperCase();
  const target = previewUrl(String(url));
  try {
    return await fetch(url, fetchOptions);
  } catch (err) {
    if (err?.name === "AbortError") {
      throw new Error(`${label} timed out after ${timeoutMs}ms (${method} ${target})`, {
        cause: err instanceof Error ? err : undefined,
      });
    }
    throw new Error(
      `${label} failed before receiving a response (${method} ${target}): ${toErrorMessage(err)}`,
      { cause: err instanceof Error ? err : undefined },
    );
  } finally {
    if (timeoutId) clearTimeout(timeoutId);
  }
}

export async function parseJsonResponse(response, label) {
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`${label} failed: ${responseStatusDetail(response)}${responseBodyDetail(text)}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`${label} did not return JSON: ${text.slice(0, 500)}`);
  }
}

export async function assertOkResponse(response, label) {
  if (response.ok) return;
  let text = "";
  try {
    text = await response.text();
  } catch (err) {
    text = `failed to read error body: ${toErrorMessage(err)}`;
  }
  throw new Error(`${label} failed: ${responseStatusDetail(response)}${responseBodyDetail(text)}`);
}

export function responseStatusDetail(response) {
  const statusText = response.statusText ? ` ${response.statusText}` : "";
  return `HTTP ${response.status}${statusText}`;
}

export function responseBodyDetail(text) {
  const body = text.trim();
  return body ? `: ${body.slice(0, 500)}` : "";
}

export async function readImagePayload(req, endpointKind) {
  const contentType = String(req.headers["content-type"] || "");
  if (/multipart\/form-data/i.test(contentType)) {
    return readMultipartImagePayload(req, contentType);
  }
  if (/application\/json/i.test(contentType) || !contentType) {
    return readJson(req);
  }
  if (endpointKind === "edit") {
    throw badRequest("images/edits requires multipart/form-data or application/json");
  }
  return readJson(req);
}

export async function readMultipartImagePayload(req, contentType) {
  const boundaryMatch = contentType.match(/boundary=(?:"([^"]+)"|([^;]+))/i);
  const boundary = boundaryMatch?.[1] || boundaryMatch?.[2];
  if (!boundary) throw badRequest("multipart/form-data missing boundary");

  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const raw = Buffer.concat(chunks).toString("latin1");
  const parts = raw.split(`--${boundary}`);
  const body = {};

  for (let part of parts) {
    if (!part || part === "--\r\n" || part === "--") continue;
    if (part.startsWith("\r\n")) part = part.slice(2);
    if (part.endsWith("\r\n")) part = part.slice(0, -2);
    if (part.endsWith("--")) part = part.slice(0, -2);
    const headerEnd = part.indexOf("\r\n\r\n");
    if (headerEnd < 0) continue;
    const headers = parseMultipartHeaders(part.slice(0, headerEnd));
    let payload = part.slice(headerEnd + 4);
    if (payload.endsWith("\r\n")) payload = payload.slice(0, -2);
    const disposition = headers["content-disposition"] || "";
    const name = disposition.match(/\bname="([^"]+)"/)?.[1];
    if (!name) continue;
    const filename = disposition.match(/\bfilename="([^"]*)"/)?.[1];
    if (filename !== undefined) {
      appendMultipartValue(body, imageFieldName(name), {
        bytes: Buffer.from(payload, "latin1"),
        contentType: headers["content-type"] || "image/png",
        filename: filename || `${name}.png`,
      });
    } else {
      appendMultipartValue(body, name, Buffer.from(payload, "latin1").toString("utf8"));
    }
  }

  return body;
}

export function parseMultipartHeaders(headerText) {
  const headers = {};
  for (const line of headerText.split("\r\n")) {
    const index = line.indexOf(":");
    if (index < 0) continue;
    headers[line.slice(0, index).trim().toLowerCase()] = line.slice(index + 1).trim();
  }
  return headers;
}

export function imageFieldName(name) {
  return ["image", "image[]", "images", "images[]"].includes(name) ? "image" : name;
}

export function appendMultipartValue(target, name, value) {
  if (target[name] === undefined) {
    target[name] = value;
    return;
  }
  if (!Array.isArray(target[name])) target[name] = [target[name]];
  target[name].push(value);
}

export async function readJson(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const raw = Buffer.concat(chunks).toString("utf8");
  if (!raw.trim()) return {};
  try {
    return JSON.parse(raw);
  } catch {
    throw new Error("request body must be JSON");
  }
}

export function sendJson(res, status, body) {
  const json = JSON.stringify(body);
  res.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(json),
  });
  res.end(json);
}

export function absoluteUrl(req, config, pathname) {
  const host = req.headers.host ?? `${config.host}:${config.port}`;
  return `http://${host}${pathname}`;
}

export function setNodeInput(workflow, nodeId, inputName, value) {
  const node = workflow[nodeId];
  if (!node || !node.inputs) {
    throw new Error(`workflow missing node ${nodeId}`);
  }
  node.inputs[inputName] = value;
}

export function setNodeInputIfPresent(workflow, nodeId, inputName, value) {
  const node = workflow[nodeId];
  if (!node?.inputs || !(inputName in node.inputs)) return;
  node.inputs[inputName] = value;
}

export function allocateWorkflowNodeId(workflow) {
  const numericIds = Object.keys(workflow)
    .map((id) => Number(id))
    .filter((id) => Number.isInteger(id) && id >= 0);
  let nextId = numericIds.length ? Math.max(...numericIds) + 1 : 1;
  while (workflow[String(nextId)]) nextId += 1;
  return String(nextId);
}

export function requiredString(value, label) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`${label} must be a non-empty string`);
  }
  return value.trim();
}

export function requiredRequestString(value, label) {
  if (typeof value !== "string" || !value.trim()) {
    throw badRequest(`${label} must be a non-empty string`);
  }
  return value.trim();
}

export function badRequest(message) {
  const err = new Error(message);
  err.statusCode = 400;
  return err;
}

export function optionalString(value) {
  return typeof value === "string" && value.trim() ? value.trim() : "";
}

export function firstNonEmpty(value) {
  if (!Array.isArray(value)) return "";
  return optionalString(value.find((item) => optionalString(item)));
}

export function intValue(value, fallback, label) {
  const number = Number(value ?? fallback);
  if (!Number.isInteger(number) || number <= 0) {
    throw new Error(`${label} must be a positive integer`);
  }
  return number;
}

export function positiveIntOrNull(value, label) {
  if (value === undefined || value === null || value === "") return null;
  const number = Number(value);
  if (!Number.isInteger(number) || number <= 0) {
    throw badRequest(`${label} must be a positive integer`);
  }
  return number;
}

export function boundedIntOrNull(value, label, min, max) {
  if (value === undefined || value === null || value === "") return null;
  const number = Number(value);
  if (!Number.isInteger(number) || number < min || number > max) {
    throw badRequest(`${label} must be an integer between ${min} and ${max}`);
  }
  return number;
}

export function boundedNumberOrNull(value, label, min, max) {
  if (value === undefined || value === null || value === "") return null;
  const number = Number(value);
  if (!Number.isFinite(number) || number < min || number > max) {
    throw badRequest(`${label} must be a number between ${min} and ${max}`);
  }
  return number;
}

export function randomSeed() {
  return Math.floor(Math.random() * 1_000_000_000_000_000);
}

export function elapsedMsForJob(job) {
  const base = job.startedAtMs || job.createdAtMs || Date.parse(job.createdAt);
  if (!Number.isFinite(base)) return 0;
  const end = job.updatedAtMs || Date.now();
  return Math.max(0, end - base);
}

export function formatTimingStatus(timing, now = Date.now()) {
  const parts = [];
  if (timing.jobId) parts.push(`job=${timing.jobId}`);
  if (timing.startedAtMs) parts.push(`elapsed=${formatElapsed(now - timing.startedAtMs)}`);
  if (timing.submittedAtMs) parts.push(`inference=${formatElapsed(now - timing.submittedAtMs)}`);
  return parts.length ? ` ${parts.join(" ")}` : "";
}

export function formatElapsedSince(startedAtMs) {
  return formatElapsed(Date.now() - startedAtMs);
}

export function formatElapsed(ms) {
  const safeMs = Math.max(0, Number(ms) || 0);
  const totalSeconds = Math.floor(safeMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h${String(minutes).padStart(2, "0")}m${String(seconds).padStart(2, "0")}s`;
  if (minutes > 0) return `${minutes}m${String(seconds).padStart(2, "0")}s`;
  return `${seconds}s`;
}

export function markRunning(job) {
  const nowMs = Date.now();
  job.status = "running";
  job.updatedAtMs = nowMs;
  job.updatedAt = new Date(nowMs).toISOString();
}

export function markFailed(job, message) {
  const nowMs = Date.now();
  job.status = "failed";
  job.error = message;
  job.updatedAtMs = nowMs;
  job.updatedAt = new Date(nowMs).toISOString();
}

export function formatComfyError(error) {
  if (!error) return "unknown error";
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}

export function toErrorMessage(err) {
  return err instanceof Error ? err.message : String(err);
}

export function filenameFromUrl(value) {
  try {
    const url = new URL(value);
    const filename = url.pathname.split("/").pop();
    return filename && filename.includes(".") ? filename : "";
  } catch {
    return "";
  }
}

export function previewUrl(value) {
  if (value.startsWith("data:")) {
    const comma = value.indexOf(",");
    const header = comma >= 0 ? value.slice(0, comma) : value.slice(0, 80);
    return `${header},...`;
  }
  return value.length > 180 ? `${value.slice(0, 180)}...` : value;
}

export function filenameForContentType(name, contentType) {
  const ext = extensionForContentType(contentType);
  if (!/\.[^.]+$/.test(name)) return `${name}${ext}`;
  return name.replace(/\.[^.]+$/, ext);
}

export function extensionForContentType(contentType) {
  if (/jpe?g/i.test(contentType)) return ".jpg";
  if (/webp/i.test(contentType)) return ".webp";
  if (/gif/i.test(contentType)) return ".gif";
  return ".png";
}

export function formatFromFilename(filename) {
  const match = filename.match(/\.([a-z0-9]+)$/i);
  const ext = match ? match[1].toLowerCase() : "mp4";
  if (ext === "m4v") return "mp4";
  return ext;
}

export function contentTypeForFormat(format) {
  switch (format) {
    case "png":
      return "image/png";
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "webp":
      return "image/webp";
    case "webm":
      return "video/webm";
    case "mov":
      return "video/quicktime";
    case "gif":
      return "image/gif";
    default:
      return "video/mp4";
  }
}

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
