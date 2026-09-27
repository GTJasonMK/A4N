// 前景粒子层：全分辨率透明画布，以 screen 混合叠在 DOM 之上。
// 粒子位置全部在顶点着色器里由时间计算，便于任意帧直接求值。
import { createGL, program } from './glutil.js';

const POINT_FS = /* glsl */ `#version 300 es
precision highp float;
in vec4 vCol;
in float vSoft;
out vec4 o;
void main() {
  vec2 d = gl_PointCoord * 2.0 - 1.0;
  float r = dot(d, d);
  if (r > 1.0) discard;
  float a = mix(exp(-r * 4.0), exp(-r * 1.6) * (1.0 - r), vSoft) * vCol.a;
  vec3 c = vCol.rgb * a;
  o = vec4(c, max(c.r, max(c.g, c.b)));
}`;

// 顶点着色器公共头：像素坐标（左上为原点）转裁剪空间
export const POINT_VS_HEAD = /* glsl */ `#version 300 es
precision highp float;
uniform vec2 uRes;
uniform float uTime;
out vec4 vCol;
out float vSoft;
vec4 toClip(vec2 px) { return vec4(px.x / uRes.x * 2.0 - 1.0, 1.0 - px.y / uRes.y * 2.0, 0.0, 1.0); }
float h11(float n) { return fract(sin(n * 127.1) * 43758.5453); }
`;

const DUST_VS = /* glsl */ `${POINT_VS_HEAD}
layout(location = 0) in vec4 aSeed;
uniform float uAmt;
uniform vec3 uColor;
uniform vec2 uDrift;
void main() {
  float z = aSeed.z;
  vec2 span = uRes + vec2(160.0);
  vec2 pos = aSeed.xy * span;
  pos.y -= uTime * mix(26.0, 6.0, z);
  pos.x += uTime * mix(10.0, 2.0, z) + sin(uTime * 0.35 + aSeed.w * 6.283) * mix(26.0, 6.0, z);
  pos += uDrift * mix(1.0, 0.2, z);
  pos = mod(pos, span) - vec2(80.0);
  float tw = 0.55 + 0.45 * sin(uTime * (0.8 + aSeed.w * 2.0) + aSeed.w * 40.0);
  bool bokeh = aSeed.w > 0.93;
  gl_PointSize = bokeh ? mix(26.0, 12.0, z) : mix(4.2, 1.6, z);
  vSoft = bokeh ? 1.0 : 0.0;
  float a = uAmt * tw * (bokeh ? 0.10 : mix(0.75, 0.30, z));
  vCol = vec4(uColor * mix(1.0, 0.7, z), a);
  gl_Position = toClip(pos);
}`;

export function createFx(canvas) {
  const gl = createGL(canvas, { alpha: true });
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.ONE, gl.ONE);

  // attribs: [{ size, data: Float32Array }]，按 location 0..n 顺序绑定
  function createPoints({ count, attribs, vs }) {
    const prog = program(gl, vs, POINT_FS);
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    attribs.forEach(({ size, data }, loc) => {
      const buf = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 0, 0);
    });
    gl.bindVertexArray(null);
    return {
      draw(uniforms) {
        gl.useProgram(prog.p);
        prog.set({ uRes: [canvas.width, canvas.height], ...uniforms });
        gl.bindVertexArray(vao);
        gl.drawArrays(gl.POINTS, 0, count);
        gl.bindVertexArray(null);
      },
    };
  }

  const DUST_N = 460;
  const seeds = new Float32Array(DUST_N * 4);
  let s = 12345;
  const rnd = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < DUST_N * 4; i++) seeds[i] = rnd();
  const dustSys = createPoints({ count: DUST_N, attribs: [{ size: 4, data: seeds }], vs: DUST_VS });

  return {
    gl,
    createPoints,
    clear() {
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    },
    dust(t, amt, { color = [1.0, 0.78, 0.45], drift = [0, 0] } = {}) {
      if (amt <= 0.001) return;
      dustSys.draw({ uTime: t, uAmt: amt, uColor: color, uDrift: drift });
    },
  };
}
