// 渲染共用：静态文件服务 + 启动走 Intel 显卡的无头 Chrome + 打开就绪页面
import http from 'node:http';
import { stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import puppeteer from 'puppeteer-core';

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.json': 'application/json',
  '.wav': 'audio/wav',
  '.mp4': 'video/mp4',
};

export function serve() {
  return new Promise((resolve, reject) => {
    const srv = http.createServer(async (req, rsp) => {
      try {
        if (!['GET', 'HEAD'].includes(req.method)) { rsp.writeHead(405); return rsp.end(); }
        let url;
        try { url = decodeURIComponent(req.url.split('?')[0]); }
        catch { rsp.writeHead(400); return rsp.end(); }
        const file = path.resolve(ROOT, '.' + (url === '/' ? '/index.html' : url));
        if (!file.startsWith(ROOT + path.sep) || url.split('/').some(p => p.startsWith('.'))) {
          rsp.writeHead(403); return rsp.end();
        }
        const info = await stat(file);
        if (!info.isFile()) { rsp.writeHead(404); return rsp.end(); }
        const range = req.headers.range;
        const match = range?.match(/^bytes=(\d+)-(\d*)$/);
        const start = match ? Number(match[1]) : 0;
        const end = match?.[2] ? Number(match[2]) : info.size - 1;
        if (range && (!match || start > end || end >= info.size || !Number.isSafeInteger(start) || !Number.isSafeInteger(end))) {
          rsp.writeHead(416, { 'Content-Range': `bytes */${info.size}` }); return rsp.end();
        }
        rsp.writeHead(range ? 206 : 200, {
          'Content-Type': TYPES[path.extname(file)] ?? 'application/octet-stream',
          'Content-Length': end - start + 1, 'Accept-Ranges': 'bytes', 'Cache-Control': 'no-store',
          ...(range ? { 'Content-Range': `bytes ${start}-${end}/${info.size}` } : {}),
        });
        if (req.method === 'HEAD') return rsp.end();
        const stream = createReadStream(file, { start, end });
        stream.on('error', error => rsp.destroy(error));
        rsp.on('close', () => stream.destroy());
        stream.pipe(rsp);
      } catch (error) {
        if (!['ENOENT', 'ENOTDIR'].includes(error.code)) console.error(error);
        rsp.writeHead(['ENOENT', 'ENOTDIR'].includes(error.code) ? 404 : 500);
        rsp.end();
      }
    });
    srv.on('error', reject);
    srv.listen(0, '127.0.0.1', () => resolve(srv));
  });
}

export function launch() {
  return puppeteer.launch({
    executablePath: '/usr/bin/google-chrome-stable',
    headless: true,
    protocolTimeout: 600000,
    args: [
      '--no-sandbox',
      '--hide-scrollbars',
      '--use-angle=gl',
      '--enable-gpu',
      '--ignore-gpu-blocklist',
      '--disable-background-timer-throttling',
      '--disable-renderer-backgrounding',
      '--disable-backgrounding-occluded-windows',
      '--force-color-profile=srgb',
      '--font-render-hinting=none',
    ],
  });
}

export async function openPage(browser, port) {
  const page = await browser.newPage();
  await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
  page.on('console', (m) => {
    if (['error', 'warn'].includes(m.type())) console.error('[页面]', m.text());
  });
  page.on('pageerror', (e) => console.error('[页面异常]', e.message));
  await page.goto(`http://127.0.0.1:${port}/index.html`, { waitUntil: 'networkidle0' });
  await page.waitForFunction('window.__ready === true || window.__error', { timeout: 120000 });
  const err = await page.evaluate(() => window.__error);
  if (err) throw new Error(`页面初始化失败：${err}`);
  return page;
}

export async function frame(page, t, file, type = 'png') {
  await page.evaluate((tt) => {
    window.__seek(tt);
    return new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  }, t);
  return page.screenshot({ path: file, type, optimizeForSpeed: true, ...(type === 'jpeg' ? { quality: 95 } : {}) });
}
