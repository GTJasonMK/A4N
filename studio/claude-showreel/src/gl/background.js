// 全局背景：低分辨率渲染的暖色墨烟（域扭曲 fbm），CSS 放大到全屏。
// 各场景通过 state 调整浓度、色调与光晕中心，背景本身不持有动画逻辑。
import { createGL, program, fullscreenTriangle, FULLSCREEN_VS, GLSL_NOISE } from './glutil.js';

const FS = /* glsl */ `#version 300 es
precision highp float;
uniform vec2 uRes;
uniform float uTime;
uniform float uAmt;
uniform vec3 uTint;
uniform vec3 uTint2;
uniform vec2 uFocus;
uniform float uGlow;
uniform float uLight;
out vec4 o;
${GLSL_NOISE}
void main() {
  vec2 uv = gl_FragCoord.xy / uRes;
  vec2 p = (gl_FragCoord.xy - 0.5 * uRes) / uRes.y;
  float t = uTime * 0.035;
  vec2 q = vec2(fbm(p * 1.4 + vec2(0.0, t)), fbm(p * 1.4 + vec2(5.2, -t * 0.8)));
  vec2 r = vec2(fbm(p * 1.4 + 2.6 * q + vec2(1.7, 9.2) + t * 1.3), fbm(p * 1.4 + 2.6 * q + vec2(8.3, 2.8) - t));
  float f = fbm(p * 1.4 + 2.8 * r);
  vec3 base = mix(vec3(0.028, 0.022, 0.018), vec3(0.060, 0.044, 0.034), f) * (0.6 + 0.4 * uLight);
  vec3 smoke = mix(uTint2, uTint, clamp(length(r), 0.0, 1.0)) * pow(f, 3.2) * 1.15 * uAmt;
  float g = exp(-2.4 * length((p - uFocus) * vec2(0.8, 1.0)));
  vec3 col = base + smoke + uTint * 0.10 * g * uGlow;
  float vig = 1.0 - 0.62 * pow(length((uv - 0.5) * vec2(1.25, 1.0)) * 1.35, 2.4);
  col *= clamp(vig, 0.0, 1.0);
  o = vec4(col, 1.0);
}`;

export function createBackground(canvas) {
  const gl = createGL(canvas);
  const prog = program(gl, FULLSCREEN_VS, FS);
  const draw = fullscreenTriangle(gl);
  const state = {};
  const reset = () =>
    Object.assign(state, {
      amt: 1,
      tint: [0.95, 0.55, 0.18],
      tint2: [0.55, 0.22, 0.12],
      focus: [0, 0],
      glow: 0.6,
      light: 1,
      fade: 1,
    });
  reset();
  return {
    state,
    reset,
    render(t) {
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.useProgram(prog.p);
      prog.set({
        uRes: [canvas.width, canvas.height],
        uTime: t,
        uAmt: state.amt,
        uTint: state.tint,
        uTint2: state.tint2,
        uFocus: state.focus,
        uGlow: state.glow,
        uLight: state.light,
      });
      draw();
      canvas.style.opacity = state.fade.toFixed(3);
    },
  };
}
