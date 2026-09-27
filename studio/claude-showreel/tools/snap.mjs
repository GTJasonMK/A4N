// 预览工具：node tools/snap.mjs 1.5 8.2 ...        → out/snap/t001.500.png
//          node tools/snap.mjs --sheet 5:8.5:0.25  → 按时间间隔拼成一张缩略图总览
import { mkdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { ROOT, serve, launch, openPage, frame } from './browser.mjs';

const args = process.argv.slice(2);
const outDir = path.join(ROOT, 'out/snap');
await mkdir(outDir, { recursive: true });
const srv = await serve();
const browser = await launch();
try {
  const page = await openPage(browser, srv.address().port);
  const name = (t) => path.join(outDir, `t${t.toFixed(3).padStart(7, '0')}.png`);
  if (args[0] === '--sheet') {
    const [a, b, step] = args[1].split(':').map(Number);
    const files = [];
    for (let t = a; t <= b + 1e-6; t += step) {
      const f = name(t);
      await frame(page, t, f);
      files.push(f);
    }
    const cols = Number(args[2] ?? 4);
    const sheet = path.join(outDir, `sheet_${a}_${b}.jpg`);
    execFileSync('montage', [...files.flatMap((f) => ['-label', path.basename(f, '.png').slice(1), f]), '-tile', `${cols}x`, '-geometry', '480x270+4+4', '-background', '#222', '-fill', '#ddd', '-pointsize', '16', sheet]);
    console.log(sheet);
  } else {
    for (const a of args) {
      const f = name(Number(a));
      await frame(page, Number(a), f);
      console.log(f);
    }
  }
} finally {
  await browser.close();
  srv.close();
}
