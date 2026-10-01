// 《潮汐之灯》程序化场景：雾港夜色、旋转灯塔光束、雾中无帆船、崖上提灯的苏晚。
// 同一场景可输出三种形态：彩色插画、漫画（网点+墨线）、"生成中"去噪效果，
// 供插画、漫画、视频、互动故事各段复用，保证"一个故事，多种形态"的视觉一致。
import { createGL, program, fullscreenTriangle, FULLSCREEN_VS, GLSL_NOISE } from './glutil.js';

const FS = /* glsl */ `#version 300 es
precision highp float;
uniform vec2 uRes;
uniform float uTime;
uniform vec3 uCam;
uniform float uBeam;
uniform float uStyle;
uniform float uWipe;
uniform float uNoise;
uniform float uExposure;
uniform float uShip;
uniform float uSeed;
uniform float uFigure;
uniform float uWarm;
out vec4 fragColor;
${GLSL_NOISE}

const float PI = 3.14159265;
const float HORIZON = -0.075;
const vec2 MOON = vec2(0.56, 0.262);
const float LHX = -0.56;
const vec2 LAMP = vec2(-0.56, 0.287);
const vec2 SHIP = vec2(0.205, -0.112);
const float WATER = -0.19;

float n1(float x) { float i = floor(x), f = fract(x); float u = f * f * (3.0 - 2.0 * f);
  return mix(fract(sin(i * 127.1) * 43758.5453), fract(sin((i + 1.0) * 127.1) * 43758.5453), u); }
float fbm1(float x) { float a = 0.5, s = 0.0; for (int i = 0; i < 5; i++) { s += a * n1(x); x *= 2.03; a *= 0.5; } return s; }

float sdBox(vec2 p, vec2 b) { vec2 d = abs(p) - b; return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0); }
float sdSeg(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0); return length(pa - ba * h); }
float sdTrap(vec2 p, float r1, float r2, float he) {
  vec2 k1 = vec2(r2, he), k2 = vec2(r2 - r1, 2.0 * he);
  p.x = abs(p.x);
  vec2 ca = vec2(p.x - min(p.x, (p.y < 0.0) ? r1 : r2), abs(p.y) - he);
  vec2 cb = p - k1 + k2 * clamp(dot(k1 - p, k2) / dot(k2, k2), 0.0, 1.0);
  float s = (cb.x < 0.0 && ca.y < 0.0) ? -1.0 : 1.0;
  return s * sqrt(min(dot(ca, ca), dot(cb, cb)));
}

float cliffTop(float x) {
  float plateau = -0.030 + 0.010 * sin(x * 7.0 + 1.0) + 0.020 * (fbm1(x * 9.0 + 3.0) - 0.5);
  float drop = smoothstep(-0.36, -0.215, x + 0.025 * (fbm1(x * 34.0) - 0.5));
  return mix(plateau, WATER - 0.03, drop);
}

// 单束光在屏幕上的投影：侧向时是细长光柱，朝向镜头时收成耀斑
float beamLobe(vec2 p, float ang) {
  vec2 v = p - LAMP;
  float len = length(v) + 1e-4;
  float c = cos(ang), s = sin(ang);
  vec2 dir = normalize(vec2(c, -0.05));
  float cosA = dot(v / len, dir);
  float halfW = 0.030 + 0.20 * max(s, 0.0) * max(s, 0.0);
  float k = clamp((cosA - cos(halfW)) / (1.0 - cos(halfW)), 0.0, 1.0);
  float lobe = k * k * (3.0 - 2.0 * k);
  float along = exp(-len * 0.75) * smoothstep(0.0, 0.035, len);
  float side = pow(abs(c), 0.7) * (s < 0.0 ? 0.55 : 1.0);
  float flare = pow(max(s, 0.0), 10.0) * (exp(-len * 38.0) * 2.2 + exp(-len * 5.5) * 0.30);
  return lobe * along * side * 1.25 + flare;
}
float beam(vec2 p) { return beamLobe(p, uBeam) + beamLobe(p, uBeam + PI); }

vec3 scene(vec2 p, float px, out float edge) {
  float t = uTime;
  edge = 0.0;
  // 天空
  float sy = clamp((p.y - HORIZON) / 0.62, 0.0, 1.0);
  vec3 col = mix(vec3(0.070, 0.100, 0.135), vec3(0.010, 0.018, 0.045), pow(sy, 0.55));
  float dm = length(p - MOON);
  col += vec3(0.30, 0.38, 0.52) * (exp(-dm * 6.5) * 0.30 + exp(-dm * 26.0) * 0.35);
  col += vec3(0.55, 0.32, 0.12) * exp(-length(p - LAMP) * 3.2) * 0.22 * uWarm;
  // 星点
  vec2 sc = p * 95.0;
  vec2 si = floor(sc);
  vec2 sf = fract(sc) - 0.5;
  float sh = hash(si + 7.31);
  vec2 so = vec2(hash(si + 1.13), hash(si + 2.27)) - 0.5;
  float star = smoothstep(0.09, 0.0, length(sf - so * 0.7)) * step(0.87, sh) * smoothstep(HORIZON + 0.06, HORIZON + 0.32, p.y);
  star *= 0.55 + 0.45 * sin(t * (1.3 + sh * 4.0) + sh * 60.0);
  // 云层（被月光勾边）
  float cn = fbm(vec2(p.x * 1.25 + t * 0.010, p.y * 3.4 + 2.0));
  float cloud = smoothstep(0.50, 0.86, cn) * smoothstep(HORIZON + 0.02, HORIZON + 0.24, p.y);
  vec3 cloudCol = vec3(0.085, 0.10, 0.135) + vec3(0.42, 0.48, 0.6) * exp(-dm * 4.5) * 0.55;
  col = mix(col, cloudCol, cloud * 0.85);
  col += vec3(0.9, 0.93, 1.0) * star * (1.0 - cloud);
  // 月亮
  float md = dm - 0.050;
  float moonMask = smoothstep(px * 1.5, -px * 1.5, md);
  vec3 moonCol = vec3(0.97, 0.95, 0.87) * (0.86 + 0.14 * fbm(p * 70.0)) * (1.0 - 0.18 * smoothstep(0.02, 0.05, dm));
  col = mix(col, moonCol, moonMask * (1.0 - cloud * 0.7));
  edge = max(edge, (1.0 - smoothstep(0.0, px * 1.8, abs(md))) * (1.0 - cloud));
  // 远山（右侧海岬）
  float mY = HORIZON + smoothstep(0.02, 0.8, p.x) * (0.030 + 0.050 * fbm1(p.x * 5.0 + 11.0)) + 0.003 * sin(p.x * 50.0);
  float mMask = smoothstep(px, -px, p.y - mY) * step(HORIZON, p.y);
  col = mix(col, mix(vec3(0.050, 0.070, 0.095), vec3(0.034, 0.048, 0.068), smoothstep(HORIZON, mY, p.y)), mMask);
  edge = mix(edge, 0.0, mMask);
  edge = max(edge, (1.0 - smoothstep(0.0, px * 1.6, abs(p.y - mY))) * step(HORIZON, p.y) * smoothstep(0.08, 0.25, p.x));
  // 海面
  float bSea = 0.0;
  if (p.y < HORIZON) {
    float dz = HORIZON - p.y;
    float persp = 1.0 / (dz + 0.012);
    vec2 sp = vec2(p.x * persp * 0.85, persp * 0.20);
    float wv = fbm3(sp + vec2(t * 0.07, -t * 0.32));
    float wl = sin(sp.y * 9.0 + wv * 5.0 - t * 1.3);
    vec3 sea = mix(vec3(0.050, 0.080, 0.110), vec3(0.008, 0.020, 0.036), smoothstep(0.0, 0.33, dz));
    sea += vec3(0.05, 0.07, 0.09) * smoothstep(0.55, 1.0, wl) * (1.0 - smoothstep(0.0, 0.4, dz)) * 0.9;
    float colW = 0.016 + dz * 0.45;
    float gm = exp(-pow((p.x - MOON.x) / colW, 2.0));
    float gl = pow(clamp(noise(vec2(sp.x * 4.0, sp.y * 7.0 - t * 1.7)), 0.0, 1.0), 5.0);
    sea += vec3(0.78, 0.82, 0.92) * gm * (0.10 + 2.8 * gl) * (0.35 + 0.65 * exp(-dz * 4.0));
    // 灯塔光在海面的暖色倒影
    float lampW = 0.012 + dz * 0.25;
    float gLamp = exp(-pow((p.x - LAMP.x) / lampW, 2.0)) * step(p.y, WATER);
    sea += vec3(1.0, 0.72, 0.38) * gLamp * (0.05 + 1.6 * gl) * 0.35 * uWarm;
    bSea = beam(vec2(p.x, HORIZON + (HORIZON - p.y) * 0.25));
    sea += vec3(1.0, 0.85, 0.6) * bSea * gl * 1.2;
    col = sea;
  }
  edge = max(edge, (1.0 - smoothstep(0.0, px * 1.4, abs(p.y - HORIZON))) * (1.0 - mMask));
  // 幽灵船：只在光束扫过时显形
  vec2 q = p - SHIP;
  float hull = sdTrap(vec2(q.x, q.y - 0.011), 0.080, 0.122, 0.011);
  hull = max(hull, -(length(q - vec2(0.0, 0.10)) - 0.085));
  float masts = min(min(sdSeg(q, vec2(-0.062, 0.02), vec2(-0.062, 0.150)), sdSeg(q, vec2(0.0, 0.02), vec2(0.0, 0.190))), sdSeg(q, vec2(0.058, 0.02), vec2(0.058, 0.140)));
  float yards = min(min(sdSeg(q, vec2(-0.090, 0.125), vec2(-0.034, 0.125)), sdSeg(q, vec2(-0.034, 0.165), vec2(0.034, 0.165))), sdSeg(q, vec2(0.034, 0.118), vec2(0.082, 0.118)));
  float rig = min(min(sdSeg(q, vec2(0.0, 0.19), vec2(0.128, 0.028)), sdSeg(q, vec2(0.0, 0.19), vec2(-0.118, 0.026))), sdSeg(q, vec2(0.122, 0.022), vec2(0.19, 0.055)));
  float shipD = min(hull, min(masts - 0.0016, min(yards - 0.0011, rig - 0.0006)));
  float shipMask = smoothstep(px, -px, shipD);
  float shipLit = clamp(beam(SHIP + vec2(0.0, 0.07)) * 1.8, 0.0, 1.0);
  float vis = clamp(0.10 + uShip + shipLit, 0.0, 1.0);
  vec3 shipCol = mix(vec3(0.06, 0.09, 0.11), vec3(0.62, 0.82, 0.86), 0.25 + 0.75 * shipLit);
  col = mix(col, shipCol, shipMask * vis * 0.88);
  col += vec3(0.30, 0.62, 0.66) * exp(-max(shipD, 0.0) * 80.0) * 0.28 * vis;
  edge = mix(edge, 0.0, shipMask * vis);
  edge = max(edge, (1.0 - smoothstep(0.0, px * 1.6, abs(shipD))) * vis);
  // 海雾：贴着海面流动，被光束照亮
  float fy = p.y - (HORIZON - 0.025);
  float band = exp(-fy * fy / 0.0045);
  float fd = fbm(vec2(p.x * 2.1 - t * 0.032, p.y * 6.5 + 1.3));
  float fog = clamp(smoothstep(0.34, 0.86, fd) * band * 0.9 + 0.10 * band, 0.0, 1.0);
  float bI = beam(p);
  vec3 fogCol = vec3(0.14, 0.175, 0.22) + vec3(0.25, 0.3, 0.36) * exp(-dm * 2.4) * 0.45 + vec3(1.0, 0.82, 0.55) * bI * 1.5;
  col = mix(col, fogCol, fog * 0.82);
  // 左侧悬崖
  float ct = cliffTop(p.x);
  float cliffD = p.y - ct;
  float cliffMask = smoothstep(px, -px, cliffD) * smoothstep(-px, px, p.y - WATER);
  float strata = fbm(p * vec2(18.0, 60.0));
  vec3 rock = vec3(0.022, 0.020, 0.024) + vec3(0.03, 0.028, 0.03) * strata;
  // Outside the cliff, an unbounded exponential becomes Inf; mix(Inf, 0) blacks out the sky.
  rock += vec3(0.25, 0.3, 0.38) * exp(min(cliffD, 0.0) / 0.004) * 0.10;
  rock += vec3(0.55, 0.32, 0.12) * exp(-length(p - vec2(LHX, -0.03)) * 9.0) * 0.30 * uWarm;
  col = mix(col, rock, cliffMask);
  edge = mix(edge, 0.0, cliffMask);
  edge = max(edge, (1.0 - smoothstep(0.0, px * 1.8, abs(cliffD))) * smoothstep(-px, px, p.y - WATER));
  // 浪花：崖脚与海面交界
  float foam = (1.0 - smoothstep(0.0, 0.006, abs(p.y - WATER))) * step(p.x, -0.2) * step(ct, WATER + 0.12);
  foam *= smoothstep(0.45, 0.9, noise(vec2(p.x * 160.0 + t * 2.0, t * 3.0)));
  col += vec3(0.5, 0.55, 0.6) * foam * 0.5;
  // 灯塔
  vec2 lq = p - vec2(LHX, 0.0);
  float tower = sdTrap(vec2(lq.x, lq.y - 0.1075), 0.036, 0.024, 0.1475);
  float gallery = sdBox(vec2(lq.x, lq.y - 0.259), vec2(0.034, 0.0045));
  float lantern = sdBox(vec2(lq.x, lq.y - 0.286), vec2(0.019, 0.022));
  float dome = max(length(vec2(lq.x, lq.y - 0.308)) - 0.021, -(lq.y - 0.308));
  float spire = sdSeg(lq, vec2(0.0, 0.325), vec2(0.0, 0.347)) - 0.0016;
  float body = min(min(tower, gallery), min(dome, spire));
  float lhMask = smoothstep(px, -px, body);
  float nx = clamp(lq.x / 0.03, -1.0, 1.0);
  vec3 towerCol = vec3(0.15, 0.14, 0.14) * (0.45 + 0.55 * (0.5 + 0.5 * nx));
  towerCol *= mix(1.0, 0.5, step(0.5, fract((lq.y + 0.03) / 0.085)));
  towerCol += vec3(0.7, 0.42, 0.15) * exp(-(0.258 - lq.y) * 12.0) * 0.30 * uWarm;
  col = mix(col, towerCol, lhMask);
  edge = mix(edge, 0.0, lhMask);
  edge = max(edge, 1.0 - smoothstep(0.0, px * 1.6, abs(body)));
  float lanMask = smoothstep(px, -px, lantern);
  vec3 glass = vec3(1.0, 0.86, 0.56) * (1.2 + 0.6 * uWarm);
  glass *= mix(1.0, 0.25, step(0.82, fract((lq.x + 0.019) / 0.0126)));
  col = mix(col, glass, lanMask);
  // 崖上提灯的苏晚（背影）
  if (uFigure > 0.5) {
    float fx0 = -0.425;
    vec2 base = vec2(fx0, cliffTop(fx0) - 0.002);
    float H = 0.060;
    vec2 f = (p - base) / H;
    float w = sin(t * 2.1) * 0.05;
    float coat = sdTrap(vec2(f.x - 0.02 - w * (0.72 - f.y), f.y - 0.42), 0.20, 0.115, 0.30);
    float legs = min(sdBox(vec2(f.x + 0.05, f.y - 0.07), vec2(0.03, 0.08)), sdBox(vec2(f.x - 0.06, f.y - 0.07), vec2(0.03, 0.08)));
    float head = length(f - vec2(0.0, 0.84)) - 0.098;
    float hair = length((f - vec2(0.06 + w * 0.6, 0.82)) * vec2(0.78, 1.15)) - 0.11;
    float arm = sdSeg(f, vec2(0.10, 0.66), vec2(0.22, 0.43)) - 0.034;
    float scarf = sdSeg(f, vec2(0.04, 0.72), vec2(0.34, 0.70 + sin(t * 3.1) * 0.05)) - 0.026 * (1.0 - clamp((f.x - 0.04) / 0.3, 0.0, 0.8));
    float fig = min(min(min(coat, legs), min(head, hair)), arm);
    float figMask = smoothstep(px / H, -px / H, fig);
    float scarfMask = smoothstep(px / H, -px / H, scarf);
    vec2 lampP = base + H * vec2(0.235, 0.36);
    float dl = length(p - lampP);
    vec3 figCol = vec3(0.020, 0.018, 0.022) + vec3(0.6, 0.35, 0.12) * exp(-dl * 60.0) * 0.5 * uWarm;
    col = mix(col, figCol, figMask);
    col = mix(col, vec3(0.42, 0.08, 0.06) + vec3(0.5, 0.2, 0.08) * exp(-dl * 40.0) * uWarm, scarfMask);
    edge = mix(edge, 0.0, max(figMask, scarfMask));
    edge = max(edge, 1.0 - smoothstep(0.0, px * 1.5, abs(min(fig * H, scarf * H))));
    col += vec3(1.0, 0.75, 0.40) * (exp(-dl * 260.0) * 1.4 + exp(-dl * 45.0) * 0.18) * uWarm;
  }
  // 光束体积光与灯室辉光
  col += vec3(1.0, 0.86, 0.60) * bI * (0.30 + 0.70 * fog) * 0.85;
  float dlamp = length(p - LAMP);
  col += vec3(1.0, 0.80, 0.48) * (exp(-dlamp * 60.0) * 1.1 + exp(-dlamp * 10.0) * 0.20) * uWarm;
  float fl = pow(max(sin(uBeam), max(sin(uBeam + PI), 0.0)), 10.0);
  col += vec3(1.0, 0.82, 0.55) * exp(-abs(p.y - LAMP.y) * 260.0) * exp(-abs(p.x - LAMP.x) * 3.5) * fl * 0.55;
  return col;
}

vec3 manga(vec3 col, float edge, vec2 frag) {
  float L = dot(col, vec3(0.299, 0.587, 0.114));
  L = pow(clamp(L * 2.6, 0.0, 1.0), 0.75);
  float cell = 5.0 * uRes.y / 1080.0 + 2.0;
  mat2 R = mat2(0.7071, -0.7071, 0.7071, 0.7071);
  vec2 hc = fract(R * frag / cell) - 0.5;
  float r = sqrt(clamp(1.0 - L, 0.0, 1.0)) * 0.64;
  float ink = smoothstep(r + 0.07, r - 0.07, length(hc));
  ink = mix(ink, 1.0, smoothstep(0.16, 0.07, L));
  ink = mix(ink, 0.0, smoothstep(0.60, 0.74, L));
  ink = max(ink, clamp(edge * 1.3, 0.0, 1.0));
  return mix(vec3(0.955, 0.945, 0.915), vec3(0.055, 0.05, 0.06), ink);
}

void main() {
  vec2 frag = gl_FragCoord.xy;
  float blk = 1.0 + 44.0 * uNoise * uNoise;
  vec2 fq = (floor(frag / blk) + 0.5) * blk;
  vec2 p = (fq - 0.5 * uRes) / uRes.y / uCam.z + uCam.xy;
  float px = blk / (uRes.y * uCam.z);
  float edge;
  vec3 col = scene(p, px, edge);
  col = 1.0 - exp(-col * uExposure);
  float mAmt = uStyle;
  float seam = 0.0;
  if (uWipe > -0.5) {
    float wx = frag.x / uRes.x + 0.05 * (fbm(frag * 0.012) - 0.5);
    mAmt = smoothstep(uWipe + 0.004, uWipe - 0.004, wx);
    seam = exp(-abs(wx - uWipe) * 90.0);
  }
  vec3 outc = col;
  if (mAmt > 0.001) outc = mix(col, manga(col, edge, frag), mAmt);
  outc += vec3(1.0, 0.78, 0.4) * seam * 0.9;
  if (uNoise > 0.001) {
    vec2 nb = floor(frag / max(blk * 0.5, 1.0));
    float fr = floor(uTime * 20.0);
    vec3 nz = vec3(hash(nb + uSeed + fr), hash(nb + uSeed + fr + 17.0), hash(nb + uSeed + fr + 41.0));
    nz = mix(vec3(dot(nz, vec3(0.33))), nz, 0.55) * vec3(0.95, 0.8, 0.62);
    outc = mix(outc, nz, clamp(uNoise * 1.05, 0.0, 1.0) * 0.9);
  }
  float vig = 1.0 - 0.35 * pow(length(frag / uRes - 0.5) * 1.4, 2.5);
  fragColor = vec4(outc * vig, 1.0);
}`;

export function createStory() {
  const canvas = document.createElement('canvas');
  canvas.width = 1920;
  canvas.height = 1080;
  const gl = createGL(canvas);
  const prog = program(gl, FULLSCREEN_VS, FS);
  const draw = fullscreenTriangle(gl);
  return {
    // target 为 2D 画布；参数缺省即全景彩色插画
    draw(target, o = {}) {
      const w = target.width;
      const h = target.height;
      gl.viewport(0, 0, w, h);
      gl.useProgram(prog.p);
      prog.set({
        uRes: [w, h],
        uTime: o.time ?? 0,
        uCam: o.cam ?? [0, 0, 1],
        uBeam: o.beam ?? 0.3,
        uStyle: o.style ?? 0,
        uWipe: o.wipe ?? -1,
        uNoise: o.noise ?? 0,
        uExposure: o.exposure ?? 1.15,
        uShip: o.ship ?? 0,
        uSeed: o.seed ?? 0,
        uFigure: o.figure ?? 1,
        uWarm: o.warm ?? 1,
      });
      draw();
      const ctx = target.getContext('2d');
      ctx.drawImage(canvas, 0, canvas.height - h, w, h, 0, 0, w, h);
    },
  };
}

// 常用机位：[中心 x, 中心 y, 缩放]
export const CAMS = {
  wide: [-0.02, 0.03, 1.0],
  lighthouse: [-0.53, 0.14, 2.6],
  ship: [0.2, -0.04, 2.9],
  figure: [-0.40, 0.0, 4.2],
  sea: [0.05, -0.02, 1.5],
};
