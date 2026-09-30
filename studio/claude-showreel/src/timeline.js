// 全片时间轴与剧本：画面与音效共用的唯一时间来源。
// 音乐为 120 BPM（一拍 0.5 秒、一小节 2 秒），场景切点都落在小节线上。
import { rng } from './core.js';

export const FPS = 60;
export const BPM = 120;
export const BEAT = 60 / BPM;
export const BAR = BEAT * 4;
export const DURATION = 123;
export const REPOSITORY = 'github.com/GTJasonMK/A4N';

// 各段落起点（秒）
export const T = {
  intro: 0,
  logo: 8,
  write: 12,
  analyze: 18,
  retrieve: 28,
  review: 38,
  drop: 48,
  image: 52,
  manga: 56,
  video: 60,
  galgame: 66,
  snapshot: 72,
  engine: 80,
  resume: 88,
  glitch: 90,
  usage: 94,
  outro: 112,
  end: DURATION,
};

// 原创演示故事《潮汐之灯》，与任何既有演示素材无关
export const STORY = {
  book: '潮汐之灯',
  chapter: '第一章　灯塔重燃',
  // 段落由普通文本与实体片段组成；cat 为分析提取的类别
  paragraphs: [
    [{ s: '雾港的灯塔', cat: 'scene' }, '熄灭了三十年。今夜零点，', { s: '它自己亮了', cat: 'hook' }, '。'],
    [
      { s: '苏晚', cat: 'person' },
      '握着',
      { s: '祖父', cat: 'person2' },
      '留下的',
      { s: '铜钥匙', cat: 'object' },
      '登上礁石。灯光扫过海面的一瞬，雾里浮出一艘',
      { s: '没有帆的船', cat: 'hook2' },
      '。',
    ],
    ['船头的人朝灯塔举起手，像在回应一个很久以前的约定。'],
  ],
};

export const CATS = {
  person: { label: '人物', color: '#f5b04a' },
  person2: { label: '人物', color: '#f5b04a' },
  object: { label: '物体', color: '#5ec8b8' },
  scene: { label: '场景', color: '#79aee6' },
  hook: { label: '伏笔', color: '#e583b0' },
  hook2: { label: '伏笔', color: '#e583b0' },
};

// 打字节奏：返回每个字符出现的时间。标点后停顿，其余字符带确定性抖动。
export function typeTimes(text, start, cps, seed = 1, { pause = 0.16, jitter = 0.35 } = {}) {
  const r = rng(seed);
  const out = [];
  let t = start;
  for (const ch of text) {
    out.push(t);
    const base = 1 / cps;
    t += base * (1 + (r() * 2 - 1) * jitter);
    if ('，。！？；：、…'.includes(ch)) t += pause;
  }
  return out;
}

// 流式生成：按 1~3 个字符一组吐出，模拟模型逐 token 输出
export function streamTimes(text, start, charsPerSec, seed = 7) {
  const r = rng(seed);
  const out = [];
  let t = start;
  let i = 0;
  const chars = [...text];
  while (i < chars.length) {
    const n = 1 + Math.floor(r() * 3);
    for (let k = 0; k < n && i < chars.length; k++, i++) out.push(t);
    t += (n / charsPerSec) * (0.7 + r() * 0.6);
  }
  return out;
}

export const plainParagraph = (p) => p.map((seg) => (typeof seg === 'string' ? seg : seg.s)).join('');

// 各场景使用的文本
export const TEXT = {
  intro: '每个故事，都从一行字开始。',
  tagline: '从一行字，到一整个世界。',
  diffKeepA: '灯光',
  diffDel: '扫过海面的一瞬，雾里',
  diffIns: '掠过海面，浓雾深处，缓缓',
  diffKeepB: '浮出一艘没有帆的船。',
  diffInstruction: '让这句更有画面感，不改剧情',
  galName: '苏晚',
  galLine: '那艘船……是在等我吗？',
  galChoices: ['举起铜灯回应', '熄灭灯塔，躲进雾里'],
  cliCall: 'a4n --output agent-json call list_projects',
  cliResult: '{"ok":true,"command":"list_projects","data":[ … ]}',
  cliResume: 'a4n workflow resume 42',
};

export const USAGE_STEPS = [
  {
    start: 94,
    title: '准备环境',
    desc: 'Node.js 22、Rust、PostgreSQL 16+；使用 Docker 开发库时需 Docker Compose',
    lines: [
      ['cp .env.example .env', '# 填写数据库地址与 JWT 密钥'],
      ['npm --prefix frontend ci', ''],
      ['./scripts/dev/dev-db.sh up', '# 本地 PostgreSQL'],
      ['./scripts/dev/setup-onnxruntime.sh', ''],
      ['./scripts/dev/setup-qdrant.sh', '# 检索索引'],
    ],
  },
  {
    start: 99,
    title: '一条命令启动',
    desc: 'Web 端或桌面端，启动前自动检查依赖',
    lines: [
      ['./scripts/run.sh --server', '# Web 端'],
      ['./scripts/run.sh', '# 桌面端'],
    ],
  },
  { start: 103, title: '配置模型', desc: '填写服务商接口、密钥与模型；保存、测试，再设为默认' },
  { start: 107.5, title: '开始创作', desc: '建项目、导入章节、分析，再写作与派生' },
];

export const FLOW = ['创建项目', '自由创作', '导入章节', '分析', '写作 · 优化', '漫画 · 视频 · 互动故事'];

export const CHAPTERS = [
  [0, '一行字的诞生'], [8, 'A4N'], [12, '写作'], [18, '章节分析'],
  [28, '自反馈检索'], [38, '作者确认'], [48, '一个故事，多种形态'],
  [52, '插画与角色'], [56, '漫画'], [60, '视频'], [66, '互动故事'],
  [72, '来源快照'], [80, '桌面、网页与命令行'], [88, '任务恢复'],
  ...USAGE_STEPS.map(s => [s.start, s.title]), [112, '从一行字，到一整个世界'],
].map(([start, title], i, all) => ({ start, end: all[i + 1]?.[0] ?? DURATION, title }));
