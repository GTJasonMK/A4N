// 场景注册表：按时间顺序排列，main.js 依次挂载
import intro from './intro.js';
import writing from './writing.js';
import { retrieve, review } from './knowledge.js';
import multimodal from './multimodal.js';
import { snapshot, engine, resume } from './workflow.js';
import { usage, outro } from './closing.js';

export const SCENES = [intro, writing, retrieve, review, multimodal, snapshot, engine, resume, usage, outro];
