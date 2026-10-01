// 各场景共用的界面组件：段落标题、鼠标指针、应用窗口外框。
import { el, splitChars, reveal, prog, E, clamp, lerp } from './core.js';

// 段落标题：编号 + 引线 + 标题逐字浮现 + 说明文字
export function makeCaption(parent, { num, kicker, title, sub, x = 120, y = 110, width = 720, titleSize }) {
  const root = el('div', 'cap', parent);
  Object.assign(root.style, { left: `${x}px`, top: `${y}px`, width: `${width}px` });
  const k = el('div', 'cap-k', root);
  const kn = el('span', 'cap-num', k, num);
  const kl = el('span', 'cap-line', k);
  const kt = el('span', '', k, kicker);
  const h = el('div', 'cap-t', root);
  if (titleSize) h.style.fontSize = `${titleSize}px`;
  const chars = splitChars(h, title);
  const s = el('div', 'cap-s', root, sub ?? '');
  return {
    root,
    update(t, tin, tout = 1e9) {
      const out = E.inCubic(prog(t, tout - 0.4, tout));
      root.style.opacity = (1 - out).toFixed(3);
      root.style.transform = `translate3d(${(-out * 40).toFixed(1)}px,0,0)`;
      root.style.filter = out > 0.01 ? `blur(${(out * 10).toFixed(1)}px)` : 'none';
      kl.style.transform = `scaleX(${E.outExpo(prog(t, tin, tin + 0.6)).toFixed(3)})`;
      reveal(kn, E.outCubic(prog(t, tin, tin + 0.4)), { dx: -12, dy: 0, blur: 4 });
      reveal(kt, E.outCubic(prog(t, tin + 0.15, tin + 0.6)), { dx: -12, dy: 0, blur: 4 });
      chars.forEach((c, i) => reveal(c, E.outExpo(prog(t, tin + 0.1 + i * 0.03, tin + 0.8 + i * 0.03)), { dy: 34, blur: 14 }));
      reveal(s, E.outCubic(prog(t, tin + 0.45, tin + 1.1)), { dy: 16, blur: 8 });
    },
  };
}

const CURSOR_SVG = `<svg viewBox="0 0 34 34" width="34" height="34"><path d="M6 3 L6 27 L12.2 21.4 L16.3 30.6 L20.4 28.8 L16.4 19.8 L24.6 19.4 Z" fill="#fbf6ee" stroke="#1a120d" stroke-width="1.8" stroke-linejoin="round"/></svg>`;

// 鼠标指针：keys 为 [{ t, x, y }]，段间 inOutCubic 插值；clicks 为点击时间
export function makeCursor(parent, keys, clicks = []) {
  const ring = el('div', 'ripple', parent);
  const node = el('div', 'cursor', parent, CURSOR_SVG);
  const pos = (t) => {
    if (t <= keys[0].t) return keys[0];
    for (let i = 0; i < keys.length - 1; i++) {
      const a = keys[i];
      const b = keys[i + 1];
      if (t <= b.t) {
        const u = E.inOutCubic(prog(t, a.t, b.t));
        return { x: lerp(a.x, b.x, u), y: lerp(a.y, b.y, u) };
      }
    }
    return keys[keys.length - 1];
  };
  return {
    pos,
    update(t, opacity = 1) {
      const p = pos(t);
      let press = 0;
      let rip = -1;
      for (const c of clicks) {
        const d = t - c;
        if (d > -0.12 && d < 0.18) press = Math.max(press, 1 - Math.abs(d + 0.0) / 0.15);
        if (d >= 0 && d < 0.6) rip = d / 0.6;
      }
      const s = 1 - 0.16 * clamp(press);
      node.style.opacity = opacity.toFixed(3);
      node.style.transform = `translate3d(${(p.x - 6).toFixed(1)}px,${(p.y - 3).toFixed(1)}px,0) scale(${s.toFixed(3)})`;
      if (rip >= 0) {
        ring.style.opacity = ((1 - rip) * 0.9 * opacity).toFixed(3);
        ring.style.transform = `translate3d(${p.x.toFixed(1)}px,${p.y.toFixed(1)}px,0) scale(${(0.2 + E.outCubic(rip) * 1.1).toFixed(3)})`;
      } else ring.style.opacity = '0';
    },
  };
}

// 应用窗口外框：返回 { root, body }
export function makeWindow(parent, { w, h, title = '', x = 0, y = 0 }) {
  const root = el('div', 'win', parent);
  Object.assign(root.style, { width: `${w}px`, height: `${h}px`, left: `${x}px`, top: `${y}px` });
  const bar = el('div', 'win-bar', root);
  for (let i = 0; i < 3; i++) el('span', 'win-dot', bar);
  el('span', 'win-title', bar, title);
  const body = el('div', '', root);
  Object.assign(body.style, { position: 'absolute', left: '0', right: '0', top: '52px', bottom: '0' });
  return { root, bar, body };
}

// 屏幕闪光：在 t0 瞬间最亮，随后衰减
export function flashAmount(t, t0, rise = 0.06, decay = 0.7) {
  if (t < t0 - rise || t > t0 + decay) return 0;
  if (t < t0) return E.inQuad(prog(t, t0 - rise, t0));
  return Math.exp(-((t - t0) / decay) * 4.5);
}
