// 01 写作 + 02 章节分析（12–28.3s）
// 写作台窗口飞入并按章纲流式生成正文；点击「分析」后扫描光带扫过正文，
// 实体逐个高亮，档案卡带着回指原文的光线飞出，最后同步状态逐项完成。
import { el, svgEl, prog, E, clamp, lerp, spring, reveal, addStyle } from '../core.js';
import { STORY, CATS, streamTimes, plainParagraph } from '../timeline.js';
import { makeCaption, makeCursor, makeWindow } from '../ui.js';

const WIN = { w: 1060, h: 700 };
const S2 = { cx: 1300, cy: 545, s: 1, ry: -9, rx: 2 };
const S3 = { cx: 575, cy: 610, s: 0.82, ry: 7, rx: 1 };
const STREAM0 = 13.5;
const CLICK = 18.0;
const SCAN = [18.35, 19.95];
const LAUNCH = [20.2, 20.6, 21.0, 21.4];
const FLY = 0.75;
const SYNC0 = 24.6;
const SYNC_ROWS = ['人物档案', '物体档案', '场景档案', '伏笔', '检索索引'];

const CARDS = [
  { key: 'person', name: '苏晚', body: '持有铜钥匙（祖父留下）· 位于灯塔下的礁石', quote: '苏晚握着祖父留下的铜钥匙登上礁石' },
  { key: 'object', name: '铜钥匙', body: '来源：祖父 · 当前由苏晚携带', quote: '祖父留下的铜钥匙' },
  { key: 'scene', name: '雾港灯塔', body: '熄灭三十年，今夜零点重新亮起', quote: '雾港的灯塔熄灭了三十年' },
  { key: 'hook2', name: '无帆之船', body: '雾中浮现，船头有人 · 尚未回收', quote: '雾里浮出一艘没有帆的船' },
];
// 次要引用：同一张卡的额外出处
const EXTRA_LINKS = [
  { ent: 'person2', card: 0 },
  { ent: 'hook', card: 2 },
];

const CSS = `
.wd-side { position:absolute; left:0; top:0; bottom:0; width:230px; border-right:1px solid var(--line); padding:22px 14px; background:rgba(0,0,0,.12); }
.wd-side h4 { font:500 15px var(--sans); color:var(--ink3); letter-spacing:.2em; margin:0 10px 14px; }
.wd-item { padding:12px 14px; border-radius:10px; margin-bottom:6px; font:400 17px var(--sans); color:var(--ink3); }
.wd-item b { display:block; font:500 18px var(--serif); color:var(--ink2); margin-bottom:3px; }
.wd-item.on { background:rgba(232,155,108,.12); box-shadow:inset 3px 0 0 var(--terra); }
.wd-item.on b { color:var(--ink); }
.wd-import { margin:14px 10px; padding:10px 12px; border:1px dashed rgba(255,230,200,.18); border-radius:10px; font:400 16px var(--sans); color:var(--ink3); text-align:center; }
.wd-main { position:absolute; left:230px; right:0; top:0; bottom:0; }
.wd-tabs { height:58px; display:flex; align-items:flex-end; gap:34px; padding:0 44px; border-bottom:1px solid var(--line); position:relative; }
.wd-tab { font:500 19px var(--sans); color:var(--ink3); padding-bottom:14px; }
.wd-tab.on { color:var(--ink); }
.wd-ul { position:absolute; bottom:-1px; height:3px; border-radius:3px; background:var(--terra); box-shadow:0 0 12px rgba(232,155,108,.8); }
.wd-state { position:absolute; right:30px; top:17px; font:400 16px var(--sans); color:var(--ink3); display:flex; align-items:center; gap:8px; }
.wd-doc { position:absolute; left:0; right:0; top:58px; bottom:44px; padding:34px 52px; overflow:hidden; }
.wd-doc h2 { font:600 30px var(--serif); color:var(--ink); letter-spacing:.06em; margin-bottom:22px; }
.wd-p { font:400 23px/1.95 var(--serif); color:#e3d8ca; text-indent:2em; margin-bottom:8px; letter-spacing:.04em; }
.ci { opacity:0; }
.ent { border-radius:4px; padding:1px 1px; background-repeat:no-repeat; background-size:0% 100%; }
.wd-foot { position:absolute; left:0; right:0; bottom:0; height:44px; border-top:1px solid var(--line); display:flex; align-items:center; padding:0 44px; gap:22px; font:400 15px var(--sans); color:var(--ink3); }
.wd-caret { position:absolute; width:12px; height:26px; background:var(--amber); border-radius:2px; box-shadow:0 0 12px rgba(245,165,36,.9); }
.wd-scan { position:absolute; left:20px; right:20px; height:3px; border-radius:3px; background:linear-gradient(90deg, rgba(255,210,138,0), #fff3dc 20%, #ffd28a 50%, #fff3dc 80%, rgba(255,210,138,0)); box-shadow:0 0 18px rgba(255,190,90,.95), 0 0 50px rgba(245,150,40,.6); }
.wd-scan::before { content:''; position:absolute; left:0; right:0; bottom:3px; height:120px; background:linear-gradient(0deg, rgba(255,190,90,.20), rgba(255,190,90,0)); }
.an-card { position:absolute; left:0; top:0; width:640px; height:146px; border-radius:16px; padding:18px 24px 0 30px;
  background:linear-gradient(180deg, rgba(44,35,29,.96), rgba(30,24,20,.96)); border:1px solid rgba(255,226,190,.12);
  box-shadow:0 30px 70px -25px rgba(0,0,0,.9); overflow:hidden; }
.an-bar { position:absolute; left:0; top:0; bottom:0; width:5px; }
.an-head { display:flex; align-items:center; gap:14px; }
.an-cat { font:500 16px var(--sans); padding:3px 12px; border-radius:999px; }
.an-name { font:600 28px var(--serif); color:var(--ink); letter-spacing:.05em; }
.an-body { font:400 18px var(--sans); color:var(--ink2); margin-top:8px; }
.an-quote { display:flex; align-items:center; gap:12px; margin-top:10px; font:400 17px var(--serif); color:var(--ink3); white-space:nowrap; }
.an-badge { font:500 14px var(--sans); color:var(--green); border:1px solid rgba(127,209,151,.45); padding:1px 9px; border-radius:999px; }
.an-sync { position:absolute; width:640px; border-radius:16px; padding:20px 26px; background:rgba(22,18,15,.97); border:1px solid rgba(255,226,190,.14); box-shadow:0 30px 70px -25px rgba(0,0,0,.9); }
.an-sync h5 { font:500 19px var(--sans); color:var(--ink); margin-bottom:12px; letter-spacing:.06em; }
.an-row { display:flex; align-items:center; gap:14px; height:38px; font:400 18px var(--sans); color:var(--ink2); }
.an-dot { width:22px; height:22px; border-radius:50%; border:2px solid rgba(255,230,200,.25); display:flex; align-items:center; justify-content:center; font:700 13px var(--sans); color:#16110d; }
.an-bar2 { flex:1; height:4px; border-radius:4px; background:rgba(255,230,200,.08); overflow:hidden; }
.an-bar2 i { display:block; height:100%; background:linear-gradient(90deg, var(--terra), var(--amber2)); }
`;

function hexA(hex, a) {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${n >> 16},${(n >> 8) & 255},${n & 255},${a})`;
}

function winTransform(st, z = 0, extra = {}) {
  const s = st.s * (extra.s ?? 1);
  return `translate3d(${(st.cx - WIN.w / 2).toFixed(2)}px,${(st.cy - WIN.h / 2).toFixed(2)}px,${z.toFixed(1)}px) rotateX(${st.rx.toFixed(3)}deg) rotateY(${st.ry.toFixed(3)}deg) scale(${s.toFixed(4)})`;
}
const mixState = (a, b, u) => ({ cx: lerp(a.cx, b.cx, u), cy: lerp(a.cy, b.cy, u), s: lerp(a.s, b.s, u), ry: lerp(a.ry, b.ry, u), rx: lerp(a.rx, b.rx, u) });

export default {
  from: 11.9,
  to: 28.3,
  async mount(root, ctx) {
    addStyle(CSS);
    this.cap2 = makeCaption(root, { num: '01', kicker: '写作', title: '从灵感和大纲起笔', sub: '也可以导入已有章节，接着写下去', x: 120, y: 380, width: 600 });
    this.cap3 = makeCaption(root, { num: '02', kicker: '章节分析', title: '每一条资料，都能回到原文', sub: '提取事实、人物状态与伏笔，同步档案并建立检索索引', x: 120, y: 70, width: 1500, titleSize: 52 });
    const persp = el('div', 'persp', root);
    const win = makeWindow(persp, { w: WIN.w, h: WIN.h, title: `${STORY.book} — 写作台` });
    this.win = win.root;
    this.win.style.transformOrigin = '50% 50%';
    const side = el('div', 'wd-side', win.body);
    el('h4', '', side, '章节');
    const items = [
      ['第 1 章　灯塔重燃', '生成中'],
      ['第 2 章　无帆之船', '章纲'],
      ['第 3 章　潮汐日志', '章纲'],
    ];
    this.sideState = null;
    items.forEach(([a, b], i) => {
      const it = el('div', `wd-item${i === 0 ? ' on' : ''}`, side, `<b>${a}</b>`);
      const st = el('span', '', it, b);
      if (i === 0) this.sideState = st;
    });
    el('div', 'wd-import', side, '＋ 导入章节');
    const main = el('div', 'wd-main', win.body);
    const tabs = el('div', 'wd-tabs', main);
    this.tabs = ['正文', '分析', '写作助手'].map((n, i) => el('div', `wd-tab${i === 0 ? ' on' : ''}`, tabs, n));
    this.ul = el('div', 'wd-ul', tabs);
    this.state = el('div', 'wd-state', tabs);
    const doc = el('div', 'wd-doc', main);
    el('h2', '', doc, STORY.chapter);
    this.doc = doc;
    // 正文：逐字 span，实体片段包一层 .ent 以便高亮
    this.chars = [];
    this.ents = {};
    for (const para of STORY.paragraphs) {
      const p = el('p', 'wd-p', doc);
      for (const seg of para) {
        const isEnt = typeof seg !== 'string';
        const host = isEnt ? el('span', 'ent', p) : p;
        if (isEnt) this.ents[seg.cat] = { node: host, cat: seg.cat };
        for (const ch of isEnt ? seg.s : seg) {
          const c = el('span', 'ci', host);
          c.textContent = ch;
          this.chars.push(c);
        }
      }
    }
    const full = STORY.paragraphs.map(plainParagraph).join('');
    this.times = streamTimes(full, STREAM0, 30, 11);
    this.caret = el('div', 'wd-caret', doc);
    this.scan = el('div', 'wd-scan', doc);
    const foot = el('div', 'wd-foot', main);
    this.words = el('span', '', foot, '字数 0');
    this.saved = el('span', '', foot, '');

    // 以窗口未变换时的布局测量：标签位置、字符位置、实体位置
    const docR = doc.getBoundingClientRect();
    this.charPos = this.chars.map((c) => {
      const r = c.getBoundingClientRect();
      return { x: r.left - docR.left, y: r.top - docR.top, w: r.width, h: r.height };
    });
    const tabR = this.tabs.map((n) => n.getBoundingClientRect());
    const tabsR = tabs.getBoundingClientRect();
    this.tabGeo = tabR.map((r) => ({ x: r.left - tabsR.left, w: r.width }));
    const textTop = Math.min(...this.charPos.map((p) => p.y));
    const textBot = Math.max(...this.charPos.map((p) => p.y + p.h));
    this.scanGeo = { top: textTop - 10, bot: textBot + 10 };
    for (const k in this.ents) {
      const r = this.ents[k].node.getBoundingClientRect();
      const yc = r.top - docR.top + r.height / 2;
      this.ents[k].hit = SCAN[0] + ((yc - this.scanGeo.top) / (this.scanGeo.bot - this.scanGeo.top)) * (SCAN[1] - SCAN[0]);
      this.ents[k].color = CATS[k].color;
    }

    // 档案卡
    this.cardLayer = el('div', 'layer', root);
    this.cards = CARDS.map((c, i) => {
      const cat = CATS[c.key];
      const n = el('div', 'an-card', this.cardLayer);
      el('div', 'an-bar', n).style.background = cat.color;
      const head = el('div', 'an-head', n);
      const chip = el('span', 'an-cat', head, cat.label);
      Object.assign(chip.style, { color: cat.color, background: hexA(cat.color, 0.14), border: `1px solid ${hexA(cat.color, 0.4)}` });
      el('span', 'an-name', head, c.name);
      el('div', 'an-body', n, c.body);
      const q = el('div', 'an-quote', n, `「${c.quote}」`);
      const badge = el('span', 'an-badge', q, '原文引用 ✓');
      return { node: n, badge, slot: { x: 1180, y: 250 + i * 162 }, launch: LAUNCH[i], ent: c.key, color: cat.color };
    });
    // 引用光线（模糊层 + 清晰层）
    const mkSvg = (blur) => {
      const s = svgEl('svg', { width: 1920, height: 1080 }, root);
      Object.assign(s.style, { position: 'absolute', left: '0', top: '0', pointerEvents: 'none', filter: blur ? 'blur(5px)' : 'none' });
      return s;
    };
    const svgBlur = mkSvg(true);
    const svgSharp = mkSvg(false);
    const links = [...this.cards.map((c, i) => ({ ent: c.ent, card: i, main: true })), ...EXTRA_LINKS.map((l) => ({ ...l, main: false }))];
    this.links = links.map((l) => {
      const color = CATS[l.ent].color;
      const wide = svgEl('path', { fill: 'none', stroke: color, 'stroke-width': l.main ? 7 : 4, opacity: 0.55 }, svgBlur);
      const thin = svgEl('path', { fill: 'none', stroke: color, 'stroke-width': l.main ? 2 : 1.4, 'stroke-linecap': 'round' }, svgSharp);
      const dot = svgEl('circle', { r: 5, fill: '#fff6e6' }, svgSharp);
      dot.style.filter = `drop-shadow(0 0 6px ${color}) drop-shadow(0 0 14px ${color})`;
      return { ...l, wide, thin, dot };
    });

    // 同步状态面板
    this.sync = el('div', 'an-sync', root);
    Object.assign(this.sync.style, { left: '1180px', top: '905px', width: '640px', padding: '14px 26px' });
    el('h5', '', this.sync, '本章资料同步状态');
    const rowWrap = el('div', '', this.sync);
    Object.assign(rowWrap.style, { display: 'flex', gap: '18px' });
    this.syncRows = SYNC_ROWS.map((name) => {
      const r = el('div', 'an-row', rowWrap);
      r.style.gap = '8px';
      const d = el('span', 'an-dot', r, '');
      el('span', '', r, name).style.fontSize = '16px';
      return { r, d };
    });

    // 鼠标：在 S3 布局下测量「分析」标签的屏幕位置
    this.win.style.transform = winTransform(S3);
    const tr = this.tabs[1].getBoundingClientRect();
    this.win.style.transform = '';
    const tx = tr.left + tr.width / 2;
    const ty = tr.top + tr.height / 2;
    this.cursor = makeCursor(root, [
      { t: 17.25, x: 1520, y: 860 },
      { t: 17.95, x: tx, y: ty },
      { t: 18.35, x: tx + 6, y: ty + 4 },
      { t: 18.9, x: tx + 150, y: ty + 260 },
    ], [CLICK]);

    // 音效登记
    ctx.cue(12.0, 'swoosh', { dur: 0.9 });
    ctx.cue(12.75, 'settle');
    const chunkStarts = [...new Set(this.times)];
    chunkStarts.forEach((t, i) => ctx.cue(t, 'stream', { i }));
    ctx.cue(16.95, 'tick', { note: 81 });
    ctx.cue(17.05, 'swish', { dur: 0.8 });
    ctx.cue(CLICK, 'click');
    ctx.cue(SCAN[0], 'scan', { dur: SCAN[1] - SCAN[0] });
    const notes = { scene: 62, hook: 69, person: 65, person2: 67, object: 72, hook2: 74 };
    for (const k in this.ents) ctx.cue(this.ents[k].hit, 'blip', { note: notes[k] + 12 });
    this.cards.forEach((c, i) => {
      ctx.cue(c.launch, 'fly', { i });
      ctx.cue(c.launch + FLY, 'land', { i });
      ctx.cue(22.4 + i * 0.3, 'tick', { note: 86 + (i % 2) * 3 });
    });
    this.syncTimes = SYNC_ROWS.map((_, i) => SYNC0 + 0.45 + i * 0.32);
    this.syncTimes.forEach((t, i) => ctx.cue(t, 'check', { i }));
    ctx.cue(27.35, 'whoosh', { dur: 0.9, dir: -1 });
  },

  update(t, ctx) {
    ctx.bg.state.amt = 0.75;
    ctx.bg.state.focus = [0.25, 0.0];
    ctx.dust = 0.35;
    this.cap2.update(t, 12.35, 17.25);
    this.cap3.update(t, 18.25, 27.45);

    // 窗口：飞入 → S2 → S3 → 缩小离场
    const inP = spring(t - 11.95, 0.95, 0.78);
    const fly = { cx: lerp(1520, S2.cx, inP), cy: lerp(600, S2.cy, inP), s: 1, ry: lerp(-42, S2.ry, inP), rx: lerp(10, S2.rx, inP) };
    const mv = E.inOutCubic(prog(t, 17.0, 17.85));
    let st = mixState(fly, S3, mv);
    const outP = E.inCubic(prog(t, 27.3, 28.2));
    const z = lerp(-1500, 0, clamp(inP)) - outP * 900;
    this.win.style.transform = winTransform(st, z);
    this.win.style.opacity = (clamp(prog(t, 11.95, 12.35)) * (1 - outP)).toFixed(3);
    this.win.style.filter = t < 12.5 ? `blur(${((1 - clamp(inP)) * 12).toFixed(1)}px)` : outP > 0.01 ? `blur(${(outP * 10).toFixed(1)}px)` : 'none';

    // 正文流式生成
    let lastVisible = -1;
    this.chars.forEach((c, i) => {
      const p = prog(t, this.times[i], this.times[i] + 0.18);
      c.style.opacity = p.toFixed(3);
      if (p > 0) lastVisible = i;
    });
    const generating = t >= STREAM0 && t < this.times[this.times.length - 1] + 0.2;
    const done = t >= this.times[this.times.length - 1] + 0.2;
    if (lastVisible >= 0 && t < 17.1) {
      const cp = this.charPos[lastVisible];
      this.caret.style.opacity = generating || (t % 0.9) < 0.5 ? '1' : '0';
      this.caret.style.transform = `translate3d(${(cp.x + cp.w + 3).toFixed(1)}px,${(cp.y + 8).toFixed(1)}px,0)`;
    } else this.caret.style.opacity = '0';
    const count = lastVisible + 1;
    this.words.textContent = `字数 ${count}`;
    this.saved.textContent = done ? '已保存' : generating ? '按章纲生成中…' : '';
    this.sideState.textContent = done ? '已完成' : '生成中';
    const analyzing = t >= CLICK && t < SCAN[1] + 0.2;
    const analyzed = t >= SCAN[1] + 0.2;
    this.state.innerHTML = analyzing ? '<span style="color:var(--amber2)">●</span> 分析中…' : analyzed ? '<span style="color:var(--green)">✓</span> 分析完成' : '';

    // 标签切换
    const tabP = E.outCubic(prog(t, CLICK, CLICK + 0.3));
    const g0 = this.tabGeo[0];
    const g1 = this.tabGeo[1];
    this.ul.style.left = `${lerp(g0.x, g1.x, tabP).toFixed(1)}px`;
    this.ul.style.width = `${lerp(g0.w, g1.w, tabP).toFixed(1)}px`;
    this.tabs[0].classList.toggle('on', tabP < 0.5);
    this.tabs[1].classList.toggle('on', tabP >= 0.5);

    // 扫描光带
    const sp = prog(t, SCAN[0], SCAN[1]);
    const scanVis = t >= SCAN[0] - 0.1 && t <= SCAN[1] + 0.25;
    this.scan.style.opacity = scanVis ? (Math.min(1, (t - SCAN[0] + 0.1) * 6) * (1 - prog(t, SCAN[1], SCAN[1] + 0.25))).toFixed(3) : '0';
    this.scan.style.top = `${lerp(this.scanGeo.top, this.scanGeo.bot, E.inOutSine(sp)).toFixed(1)}px`;

    // 实体高亮：马克笔从左向右刷开
    for (const k in this.ents) {
      const e = this.ents[k];
      const hp = E.outCubic(prog(t, e.hit, e.hit + 0.35));
      const pulse = Math.exp(-Math.max(0, t - e.hit) * 4) * (t >= e.hit ? 1 : 0);
      e.node.style.backgroundImage = `linear-gradient(90deg, ${hexA(e.color, 0.30)}, ${hexA(e.color, 0.22)})`;
      e.node.style.backgroundSize = `${(hp * 100).toFixed(1)}% 100%`;
      e.node.style.boxShadow = hp > 0 ? `inset 0 -2px 0 ${hexA(e.color, 0.9 * hp)}, 0 0 ${(18 * pulse).toFixed(1)}px ${hexA(e.color, 0.8 * pulse)}` : 'none';
      e.node.style.color = hp > 0 ? '#fff4e6' : '';
    }

    // 档案卡飞出：从实体位置沿弧线飞到右侧槽位
    const fade = 1 - prog(t, 27.0, 27.55);
    const entRect = {};
    for (const k in this.ents) entRect[k] = this.ents[k].node.getBoundingClientRect();
    this.cards.forEach((c, i) => {
      const r = entRect[c.ent];
      const u = clamp(spring(t - c.launch, 1.25, 0.72));
      const lin = prog(t, c.launch, c.launch + FLY);
      const sx = r.left + r.width / 2 - 320;
      const sy = r.top + r.height / 2 - 73;
      const x = lerp(sx, c.slot.x, u);
      const y = lerp(sy, c.slot.y, u) - Math.sin(Math.PI * clamp(lin)) * 90;
      const s = lerp(0.18, 1, clamp(u));
      c.node.style.transform = `translate3d(${x.toFixed(1)}px,${y.toFixed(1)}px,0) scale(${s.toFixed(4)})`;
      c.node.style.opacity = (t >= c.launch ? Math.min(1, (t - c.launch) * 5) * fade : 0).toFixed(3);
      c.cur = { x: x + 320 * (1 - s), y: y + 73, s };
      reveal(c.badge, E.outBack(prog(t, 22.4 + i * 0.3, 22.75 + i * 0.3)), { dy: 0, dx: -10, blur: 3, scale: 0.4 });
    });

    // 引用光线：从正文实体连到卡片左缘，光点沿线流动
    this.links.forEach((l, j) => {
      const c = this.cards[l.card];
      const r = entRect[l.ent];
      const launch = l.main ? c.launch : c.launch + 1.2 + j * 0.2;
      const drawP = E.outCubic(prog(t, launch + (l.main ? 0.05 : 0), launch + FLY + 0.2));
      if (t < launch || fade <= 0) {
        l.thin.style.opacity = '0';
        l.wide.style.opacity = '0';
        l.dot.style.opacity = '0';
        return;
      }
      const x0 = r.right + 2;
      const y0 = r.top + r.height / 2;
      const x1 = c.cur.x - 2;
      const y1 = c.cur.y;
      const dx = Math.max(80, (x1 - x0) * 0.5);
      const d = `M${x0.toFixed(1)},${y0.toFixed(1)} C${(x0 + dx).toFixed(1)},${y0.toFixed(1)} ${(x1 - dx).toFixed(1)},${y1.toFixed(1)} ${x1.toFixed(1)},${y1.toFixed(1)}`;
      l.thin.setAttribute('d', d);
      l.wide.setAttribute('d', d);
      const len = l.thin.getTotalLength();
      for (const pth of [l.thin, l.wide]) {
        pth.style.strokeDasharray = `${len} ${len}`;
        pth.style.strokeDashoffset = `${((1 - drawP) * len).toFixed(1)}`;
        pth.style.opacity = ((l.main ? 0.95 : 0.6) * fade).toFixed(3);
      }
      l.wide.style.opacity = ((l.main ? 0.5 : 0.3) * fade).toFixed(3);
      const period = 1.3;
      const ph = ((t - launch - FLY + j * 0.37) % period) / period;
      if (t > launch + FLY && ph >= 0) {
        const pt = l.thin.getPointAtLength(len * ph);
        l.dot.setAttribute('cx', pt.x.toFixed(1));
        l.dot.setAttribute('cy', pt.y.toFixed(1));
        l.dot.style.opacity = (Math.sin(Math.PI * ph) * fade).toFixed(3);
      } else l.dot.style.opacity = '0';
    });

    // 同步状态
    const syncIn = E.outCubic(prog(t, SYNC0, SYNC0 + 0.45));
    reveal(this.sync, syncIn * fade, { dy: 30, blur: 8 });
    this.syncRows.forEach((row, i) => {
      const ok = t >= this.syncTimes[i];
      const p = E.outBack(prog(t, this.syncTimes[i], this.syncTimes[i] + 0.3));
      row.d.textContent = ok ? '✓' : '';
      row.d.style.background = ok ? `rgba(127,209,151,${p.toFixed(3)})` : 'transparent';
      row.d.style.borderColor = ok ? 'rgba(127,209,151,.9)' : 'rgba(255,230,200,.25)';
      row.d.style.transform = `scale(${ok ? (0.6 + 0.4 * p).toFixed(3) : 1})`;
      row.r.style.color = ok ? '#efe6da' : '';
    });

    this.cursor.update(t, clamp(prog(t, 17.2, 17.45)) * (1 - prog(t, 18.6, 18.95)));
  },
};
