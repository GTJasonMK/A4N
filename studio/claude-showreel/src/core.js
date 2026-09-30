// 动画核心工具：所有画面状态都必须是时间 t 的纯函数，禁止 Math.random 与实时动画，
// 这样任意帧可以独立渲染、多进程并行且结果可复现。

export const clamp = (x, a = 0, b = 1) => (x < a ? a : x > b ? b : x);
export const lerp = (a, b, t) => a + (b - a) * t;
// t 在 [a, b] 区间内的归一化进度
export const prog = (t, a, b) => clamp((t - a) / (b - a));
// 进入-保持-退出的包络：[a, a+fi] 淡入，[b-fo, b] 淡出
export const window01 = (t, a, b, fi = 0.4, fo = 0.4) =>
  Math.min(prog(t, a, a + fi), 1 - prog(t, b - fo, b));

export const E = {
  linear: (t) => t,
  inQuad: (t) => t * t,
  outQuad: (t) => 1 - (1 - t) * (1 - t),
  inOutQuad: (t) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2),
  inCubic: (t) => t * t * t,
  outCubic: (t) => 1 - (1 - t) ** 3,
  inOutCubic: (t) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2),
  outQuart: (t) => 1 - (1 - t) ** 4,
  outQuint: (t) => 1 - (1 - t) ** 5,
  inQuart: (t) => t ** 4,
  inExpo: (t) => (t <= 0 ? 0 : 2 ** (10 * t - 10)),
  outExpo: (t) => (t >= 1 ? 1 : 1 - 2 ** (-10 * t)),
  inOutExpo: (t) =>
    t <= 0 ? 0 : t >= 1 ? 1 : t < 0.5 ? 2 ** (20 * t - 10) / 2 : (2 - 2 ** (-20 * t + 10)) / 2,
  inOutSine: (t) => -(Math.cos(Math.PI * t) - 1) / 2,
  outSine: (t) => Math.sin((t * Math.PI) / 2),
  outBack: (t, s = 1.70158) => 1 + (s + 1) * (t - 1) ** 3 + s * (t - 1) ** 2,
  inBack: (t, s = 1.70158) => (s + 1) * t ** 3 - s * t * t,
};

// 阻尼弹簧阶跃响应（0 → 1），t 为秒；freq 越大越快，damping < 1 时会过冲回弹
export function spring(t, freq = 2.0, damping = 0.6) {
  if (t <= 0) return 0;
  const w = 2 * Math.PI * freq;
  const z = damping;
  if (z >= 1) return 1 - Math.exp(-w * t) * (1 + w * t);
  const wd = w * Math.sqrt(1 - z * z);
  return 1 - Math.exp(-z * w * t) * (Math.cos(wd * t) + ((z * w) / wd) * Math.sin(wd * t));
}

// mulberry32：可复现的伪随机序列
export function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// 基于整数的无状态哈希噪声，用于逐帧抖动（同一 t 永远得到同一值）
export function hash1(n) {
  const s = Math.sin(n * 127.1 + 311.7) * 43758.5453;
  return s - Math.floor(s);
}
export function noise1(x) {
  const i = Math.floor(x);
  const f = x - i;
  const u = f * f * (3 - 2 * f);
  return lerp(hash1(i), hash1(i + 1), u) * 2 - 1;
}

export function el(tag, cls, parent, html) {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (html != null) n.innerHTML = html;
  if (parent) parent.appendChild(n);
  return n;
}

export function svgEl(tag, attrs, parent) {
  const n = document.createElementNS('http://www.w3.org/2000/svg', tag);
  for (const k in attrs) n.setAttribute(k, attrs[k]);
  if (parent) parent.appendChild(n);
  return n;
}

// 3D 变换字符串；角度单位为度
export function tf({ x = 0, y = 0, z = 0, s = 1, sx = 1, sy = 1, rx = 0, ry = 0, rz = 0 } = {}) {
  return `translate3d(${x.toFixed(2)}px,${y.toFixed(2)}px,${z.toFixed(2)}px) rotateX(${rx.toFixed(3)}deg) rotateY(${ry.toFixed(3)}deg) rotateZ(${rz.toFixed(3)}deg) scale3d(${(s * sx).toFixed(4)},${(s * sy).toFixed(4)},1)`;
}

// 常用的"浮现"效果：位移 + 模糊 + 透明度，p 为 0..1
export function reveal(node, p, { dy = 24, dx = 0, blur = 10, scale = 0 } = {}) {
  const q = clamp(p);
  node.style.opacity = q.toFixed(3);
  const b = (1 - q) * blur;
  node.style.filter = b > 0.05 ? `blur(${b.toFixed(2)}px)` : 'none';
  node.style.transform = `translate3d(${((1 - q) * dx).toFixed(2)}px,${((1 - q) * dy).toFixed(2)}px,0) scale(${(1 - (1 - q) * scale).toFixed(4)})`;
}

// 把文本拆成逐字 span，返回 span 数组（用于逐字动画与打字效果）
export function splitChars(node, text) {
  node.textContent = '';
  return [...text].map((ch) => {
    const s = el('span', 'ch', node);
    s.textContent = ch;
    return s;
  });
}

export function show(node, visible) {
  const v = visible ? '' : 'none';
  if (node.style.display !== v) node.style.display = v;
}

// 以 t 为参数的逐字错峰浮现
export function staggerReveal(spans, t, start, step, dur, opts) {
  spans.forEach((s, i) => reveal(s, E.outCubic(prog(t, start + i * step, start + i * step + dur)), opts));
}

export const fmt = (n) => n.toLocaleString('zh-CN');

// 场景私有样式：挂载时注入一次
export function addStyle(css) {
  const s = document.createElement('style');
  s.textContent = css;
  document.head.appendChild(s);
}
