// 序幕与标题（0–12.2s）：逐字打出开场句 → 文字碎成金色粒子 → 旋涡汇聚成 Logo（8.0s 冲击）→ 副标题。
import { el, prog, E, clamp, lerp, rng, reveal, splitChars } from '../core.js';
import { TEXT, typeTimes } from '../timeline.js';
import { POINT_VS_HEAD } from '../gl/fx.js';
import { flashAmount } from '../ui.js';

const N = 16000;
const LOGO = { cx: 960, cy: 420, size: 600 };
const REL0 = 5.0; // 粒子开始脱离字形
const CONV0 = 6.0; // 开始旋涡汇聚
const ARRIVE = 8.0; // 落位（小节线）
const TYPE = typeTimes(TEXT.intro, 1.2, 5.6, 3, { pause: 0.28, jitter: 0.3 });
const SUBS = ['小说写作', '漫画', '视频', '互动故事'];

const VS = /* glsl */ `${POINT_VS_HEAD}
layout(location = 0) in vec2 aStart;
layout(location = 1) in vec2 aTarget;
layout(location = 2) in vec4 aSeed;
layout(location = 3) in vec3 aColor;
uniform vec2 uCenter;
uniform float uAlpha;
uniform float uFade;
const float PI = 3.14159265;
void main() {
  float t = uTime;
  float rel = ${REL0.toFixed(2)} + clamp((aStart.x - 480.0) / 960.0, 0.0, 1.0) * 0.85 + aSeed.x * 0.12;
  float conv = ${CONV0.toFixed(2)} + aSeed.y * 0.55;
  float ta = clamp(t - rel, 0.0, max(conv - rel, 0.0));
  vec2 vel = vec2((aSeed.z - 0.5) * 70.0, -26.0 - aSeed.w * 64.0);
  vec2 A = aStart + vel * ta + vec2(sin(ta * 3.1 + aSeed.x * 20.0), cos(ta * 2.6 + aSeed.y * 20.0)) * 7.0 * min(ta, 1.0);
  vec2 pos = A;
  float speed = 0.0;
  if (t > conv) {
    float u = clamp((t - conv) / (${ARRIVE.toFixed(2)} - conv), 0.0, 1.0);
    float e = u * u;
    vec2 s0 = A - uCenter, s1 = aTarget - uCenter;
    float r0 = length(s0), r1 = length(s1);
    float a0 = atan(s0.y, s0.x), a1 = atan(s1.y, s1.x);
    float da = mod(a1 - a0 + PI, 2.0 * PI) - PI;
    float r = mix(r0, r1, e) * (1.0 - 0.30 * sin(PI * e));
    // 角度偏移与半径成正比 → 卷出螺旋臂；sin 包络保证起止两端连续
    float ang = a0 + (da - 2.0 * PI) * e - 2.6 * (r - r1) / 420.0 * sin(PI * e);
    pos = uCenter + r * vec2(cos(ang), sin(ang));
    speed = u;
  }
  float after = t - ${ARRIVE.toFixed(2)};
  if (after > 0.0) {
    vec2 dir = normalize(aTarget - uCenter + vec2(0.001));
    pos = aTarget + dir * 16.0 * exp(-after * 8.0) * sin(after * 34.0 + aSeed.x * 3.0);
    pos += vec2(sin(t * 7.0 + aSeed.z * 40.0), cos(t * 6.0 + aSeed.w * 40.0)) * 0.8;
  }
  float heat = smoothstep(rel - 0.02, rel + 0.25, t);
  vec3 ink = vec3(0.94, 0.90, 0.84);
  vec3 hot = vec3(1.0, 0.86, 0.58);
  vec3 c = mix(ink, hot, heat);
  c = mix(c, vec3(1.0, 0.95, 0.85), speed * speed);
  c = mix(c, aColor * 1.35, smoothstep(0.0, 0.25, after));
  bool spark = aSeed.w > 0.965;
  gl_PointSize = (spark ? 4.2 : mix(1.7, 2.9, aSeed.z)) * (1.0 + 0.5 * heat * (1.0 - speed));
  vSoft = spark ? 1.0 : 0.0;
  float ignite = smoothstep(rel - 0.03, rel + 0.08, t) * mix(0.35, 1.0, smoothstep(rel + 0.05, rel + 0.45, t));
  float a = ignite * uAlpha * uFade * (spark ? 1.0 : 0.85);
  vCol = vec4(c, a);
  gl_Position = toClip(pos);
}`;

function loadImage(src) {
  return new Promise((res, rej) => {
    const img = new Image();
    img.onload = () => res(img);
    img.onerror = rej;
    img.src = src;
  });
}

export default {
  from: 0,
  to: 12.25,
  async mount(root, ctx) {
    this.world = el('div', 'layer', root);
    // 开场句
    const line = el('div', 'abs', this.world);
    Object.assign(line.style, {
      left: '0', right: '0', top: '472px', textAlign: 'center',
      font: '300 64px/96px "Noto Serif CJK SC"', letterSpacing: '0.12em', color: '#efe6da',
    });
    this.chars = splitChars(line, TEXT.intro);
    this.caret = el('div', 'abs', this.world);
    Object.assign(this.caret.style, {
      width: '3px', height: '62px', background: '#f5a524', top: '489px', left: '0',
      boxShadow: '0 0 14px rgba(245,165,36,.9), 0 0 36px rgba(245,120,20,.5)',
    });
    this.charRects = this.chars.map((c) => c.getBoundingClientRect());

    // Logo 组
    this.logoGroup = el('div', 'layer', this.world);
    this.logo = el('img', 'abs', this.logoGroup);
    this.logo.src = 'assets/logo.svg';
    Object.assign(this.logo.style, {
      width: `${LOGO.size}px`, height: `${LOGO.size}px`,
      left: `${LOGO.cx - LOGO.size / 2}px`, top: `${LOGO.cy - LOGO.size / 2}px`,
    });
    this.word = el('div', 'abs', this.logoGroup, 'A4N');
    Object.assign(this.word.style, {
      left: '0', right: '0', top: '600px', textAlign: 'center',
      font: '300 46px "Noto Sans CJK SC"', color: '#efe6da', paddingLeft: '0.9em',
    });
    this.sub = el('div', 'abs', this.logoGroup);
    Object.assign(this.sub.style, {
      left: '0', right: '0', top: '684px', textAlign: 'center',
      font: '300 32px "Noto Serif CJK SC"', color: '#c9b8a6', letterSpacing: '0.28em',
    });
    this.subParts = [];
    SUBS.forEach((s, i) => {
      if (i) {
        const dot = el('span', 'ch', this.sub, ' · ');
        dot.style.color = '#f5a524';
        this.subParts.push(dot);
      }
      this.subParts.push(el('span', 'ch', this.sub, s));
    });
    // 冲击波与横向光晕
    this.ring = el('div', 'abs', this.world);
    Object.assign(this.ring.style, {
      width: '400px', height: '400px', left: `${LOGO.cx - 200}px`, top: `${LOGO.cy - 200}px`, borderRadius: '50%',
      border: '3px solid rgba(255,214,150,.9)', boxShadow: '0 0 40px rgba(255,170,60,.8), inset 0 0 40px rgba(255,170,60,.6)',
    });
    this.streak = el('div', 'abs', this.world);
    Object.assign(this.streak.style, {
      width: '1800px', height: '8px', left: `${LOGO.cx - 900}px`, top: `${LOGO.cy - 4}px`, borderRadius: '8px',
      background: 'linear-gradient(90deg, rgba(255,150,40,0), rgba(255,180,90,.55) 30%, rgba(255,245,225,1) 50%, rgba(255,180,90,.55) 70%, rgba(255,150,40,0))',
      filter: 'blur(3px)',
    });

    // 采样：字形像素 → 粒子起点；Logo 像素 → 粒子终点
    const W = 1920;
    const H = 1080;
    const cv = document.createElement('canvas');
    cv.width = W;
    cv.height = H;
    const g = cv.getContext('2d', { willReadFrequently: true });
    g.font = '300 64px "Noto Serif CJK SC"';
    g.fillStyle = '#fff';
    const m = g.measureText('汉');
    const asc = m.fontBoundingBoxAscent;
    const desc = m.fontBoundingBoxDescent;
    this.charRects.forEach((r, i) => {
      const base = r.top + (r.height - (asc + desc)) / 2 + asc;
      g.fillText(this.chars[i].textContent, r.left, base);
    });
    let data = g.getImageData(0, 0, W, H).data;
    const src = [];
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (data[(y * W + x) * 4 + 3] > 110) src.push(x, y);
    g.clearRect(0, 0, W, H);
    const img = await loadImage('assets/logo.svg');
    g.drawImage(img, LOGO.cx - LOGO.size / 2, LOGO.cy - LOGO.size / 2, LOGO.size, LOGO.size);
    data = g.getImageData(0, 0, W, H).data;
    const dst = [];
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const k = (y * W + x) * 4;
        if (data[k + 3] > 120) dst.push(x, y, data[k] / 255, data[k + 1] / 255, data[k + 2] / 255);
      }
    const r = rng(20260927);
    const pickS = [];
    const pickD = [];
    for (let i = 0; i < N; i++) {
      const a = Math.floor(r() * (src.length / 2)) * 2;
      pickS.push([src[a] + r() - 0.5, src[a + 1] + r() - 0.5]);
      const b = Math.floor(r() * (dst.length / 5)) * 5;
      pickD.push(dst.slice(b, b + 5));
    }
    // 按 x 排序配对：左侧字形飞向 Logo 左侧，旋涡更有秩序
    pickS.sort((a, b) => a[0] + a[1] * 0.15 - (b[0] + b[1] * 0.15));
    pickD.sort((a, b) => a[0] + a[1] * 0.15 - (b[0] + b[1] * 0.15));
    const aStart = new Float32Array(N * 2);
    const aTarget = new Float32Array(N * 2);
    const aSeed = new Float32Array(N * 4);
    const aColor = new Float32Array(N * 3);
    for (let i = 0; i < N; i++) {
      aStart.set(pickS[i], i * 2);
      aTarget.set([pickD[i][0] + r() - 0.5, pickD[i][1] + r() - 0.5], i * 2);
      aSeed.set([r(), r(), r(), r()], i * 4);
      aColor.set([pickD[i][2], pickD[i][3], pickD[i][4]], i * 3);
    }
    this.particles = ctx.fx.createPoints({
      count: N,
      vs: VS,
      attribs: [
        { size: 2, data: aStart },
        { size: 2, data: aTarget },
        { size: 4, data: aSeed },
        { size: 3, data: aColor },
      ],
    });
    // 每个字的脱离时间与着色器公式一致（取字中心）
    this.charRelease = this.charRects.map((rc) => REL0 + clamp((rc.left + rc.width / 2 - 480) / 960) * 0.85);

    // 音效登记
    TYPE.forEach((t, i) => ctx.cue(t, 'key', { i, soft: 1 }));
    ctx.cue(REL0, 'shimmer', { dur: 1.4 });
    ctx.cue(CONV0, 'swirl', { dur: ARRIVE - CONV0 });
    ctx.cue(ARRIVE, 'reverseTo', { dur: 1.6 });
    ctx.cue(ARRIVE, 'impact', { size: 1 });
    [9.0, 9.5, 10.0, 10.5].forEach((t, i) => ctx.cue(t, 'bell', { note: [69, 72, 74, 77][i], vel: 0.55 }));
    ctx.cue(11.3, 'whoosh', { dur: 1.0, dir: 1 });
  },

  update(t, ctx) {
    // 背景与尘埃
    ctx.bg.state.amt = lerp(0.0, 0.55, E.inOutSine(prog(t, 0, 4))) + 0.45 * prog(t, 7.9, 8.4);
    ctx.bg.state.glow = 0.3 + 1.2 * flashAmount(t, ARRIVE, 0.1, 2.5);
    ctx.bg.state.focus = [0, 0.1];
    ctx.dust = 0.1 + 0.35 * prog(t, 1, 5) + 0.25 * prog(t, 8, 9);

    // 开场打字
    const typedN = TYPE.filter((x) => x <= t).length;
    this.chars.forEach((c, i) => {
      const tt = TYPE[i];
      const p = E.outCubic(prog(t, tt, tt + 0.12));
      const gone = prog(t, this.charRelease[i] - 0.04, this.charRelease[i] + 0.22);
      c.style.opacity = (p * (1 - gone)).toFixed(3);
      c.style.transform = `translateY(${((1 - p) * 8).toFixed(2)}px)`;
      const glow = Math.exp(-Math.max(0, t - tt) * 3.5) * p;
      c.style.textShadow = `0 0 ${(6 + 20 * glow).toFixed(1)}px rgba(255,190,110,${(0.15 + 0.7 * glow).toFixed(3)})`;
    });
    const lastRect = typedN > 0 ? this.charRects[typedN - 1] : null;
    const cx = lastRect ? lastRect.right - 4 : this.charRects[0].left - 10;
    const typing = t >= TYPE[0] && t <= TYPE[TYPE.length - 1] + 0.1;
    const blink = typing || (t % 1.0) < 0.55;
    const caretVis = prog(t, 0.2, 0.5) * (1 - prog(t, 4.9, 5.2));
    this.caret.style.opacity = (blink ? caretVis : 0).toFixed(3);
    this.caret.style.transform = `translateX(${cx.toFixed(1)}px)`;

    // 粒子（多次采样叠出运动拖尾）
    const fade = 1 - prog(t, ARRIVE + 0.05, ARRIVE + 0.9);
    if (t > REL0 - 0.1 && fade > 0) {
      const trail = [0, 1 / 240, 2 / 240, 3 / 240];
      const alphas = [1, 0.45, 0.25, 0.12];
      trail.forEach((dt, k) =>
        this.particles.draw({ uTime: t - dt, uCenter: [LOGO.cx, LOGO.cy], uAlpha: alphas[k], uFade: fade }),
      );
    }

    // Logo 落位与闪光
    const fl = flashAmount(t, ARRIVE, 0.06, 1.1);
    ctx.flash = fl * 0.85;
    const lp = E.outCubic(prog(t, ARRIVE - 0.02, ARRIVE + 0.6));
    this.logo.style.opacity = lp.toFixed(3);
    const bright = 1 + 1.6 * Math.exp(-Math.max(0, t - ARRIVE) * 3.0) * lp;
    const breathe = 0.5 + 0.5 * Math.sin((t - ARRIVE) * 2.2);
    this.logo.style.filter = `brightness(${bright.toFixed(3)}) drop-shadow(0 0 ${(18 + 10 * breathe).toFixed(1)}px rgba(245,158,27,${(0.45 + 0.15 * breathe).toFixed(3)}))`;
    const ringP = prog(t, ARRIVE, ARRIVE + 1.1);
    this.ring.style.opacity = (t >= ARRIVE ? (1 - E.outQuad(ringP)) * 0.9 : 0).toFixed(3);
    this.ring.style.transform = `scale(${(0.35 + E.outExpo(ringP) * 3.6).toFixed(3)})`;
    const stP = prog(t, ARRIVE - 0.05, ARRIVE + 1.4);
    this.streak.style.opacity = (t >= ARRIVE - 0.05 ? Math.sin(Math.PI * Math.min(1, stP * 1.6)) * (1 - stP) * 1.4 : 0).toFixed(3);
    this.streak.style.transform = `scaleX(${(0.2 + E.outExpo(stP) * 1.0).toFixed(3)})`;

    const wp = E.outCubic(prog(t, 8.3, 9.4));
    this.word.style.opacity = wp.toFixed(3);
    this.word.style.letterSpacing = `${lerp(1.6, 0.9, wp).toFixed(3)}em`;
    this.word.style.filter = `blur(${((1 - wp) * 8).toFixed(2)}px)`;
    let wi = 0;
    this.subParts.forEach((s) => {
      if (s.textContent.includes('·')) {
        reveal(s, E.outCubic(prog(t, 9.0 + (wi - 0.5) * 0.5, 9.3 + (wi - 0.5) * 0.5)), { dy: 0, blur: 4 });
        return;
      }
      const t0 = 9.0 + wi * 0.5;
      reveal(s, E.outExpo(prog(t, t0 - 0.05, t0 + 0.6)), { dy: 22, blur: 12 });
      wi++;
    });

    // 整体镜头：缓慢推近，结尾穿过 Logo 进入下一段
    const push = 1 + 0.035 * E.inOutSine(prog(t, 0, 11.3));
    const outP = E.inExpo(prog(t, 11.3, 12.2));
    this.world.style.transform = `scale(${(push * (1 + outP * 1.8)).toFixed(4)})`;
    this.world.style.transformOrigin = `${LOGO.cx}px ${LOGO.cy + 60}px`;
    this.world.style.opacity = (1 - outP).toFixed(3);
    this.world.style.filter = outP > 0.01 ? `blur(${(outP * 16).toFixed(1)}px)` : 'none';
  },
};
