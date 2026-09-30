// Deterministic PNG frames → bounded, resumable H.264 chunks.
import { readdir, readFile, writeFile, mkdir, rename, access, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { parseArgs } from 'node:util';
import path from 'node:path';
import { ROOT, serve, launch, openPage, frame } from './browser.mjs';
import { FPS, DURATION } from '../src/timeline.js';

const {values}=parseArgs({options:{from:{type:'string',default:'0'},to:{type:'string',default:String(DURATION)},workers:{type:'string',default:'2'}}});
const from=Math.round(Number(values.from)*FPS),to=Math.round(Number(values.to)*FPS),workers=Number(values.workers);
if(!Number.isFinite(from)||!Number.isFinite(to)||from<0||to>FPS*DURATION||from>=to||!Number.isInteger(workers)||workers<1||workers>4)
  throw new Error('Use 0 ≤ from < to ≤ 123 and 1–4 workers');
const source=['index.html','tools/browser.mjs','tools/render.mjs'];
for(const dir of ['src','assets']) for(const name of await readdir(dir,{recursive:true})) {
  const file=path.join(dir,name);if((await stat(file)).isFile()) source.push(file);
}
const hash=createHash('sha256').update(JSON.stringify({from,to,FPS}));
for(const file of source.sort()) hash.update(file).update(await readFile(file));
const signature=hash.digest('hex');
const dest=path.join(ROOT,'out/render',signature.slice(0,16));
await mkdir(dest,{recursive:true});
const jobs=[];
for(let start=from;start<to;start+=300) jobs.push({start,end:Math.min(to,start+300),file:`${String(jobs.length).padStart(4,'0')}.mp4`});
let next=0,completed=0;
const started=Date.now();
const server=await serve();
const browsers=[];

function probe(file) {
  return JSON.parse(execFileSync('ffprobe',['-v','error','-select_streams','v:0','-show_entries','stream=nb_frames,width,height,r_frame_rate','-of','json',file],{encoding:'utf8'})).streams[0];
}

async function encode(page,job) {
  const target=path.join(dest,job.file);
  let exists;
  try {await access(target);exists=true;} catch(e) {if(e.code!=='ENOENT')throw e;exists=false;}
  if(exists) {
    const p=probe(target);
    if(Number(p.nb_frames)!==job.end-job.start||p.width!==1920||p.height!==1080||p.r_frame_rate!=='60/1')
      throw new Error(`Cached chunk failed validation: ${target}`);
    return;
  }
  const partial=target.replace('.mp4','.partial.mp4');
  const ff=spawn('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','image2pipe','-c:v','png','-probesize','32','-analyzeduration','0','-framerate',String(FPS),'-i','pipe:0',
    '-an','-vf','scale=in_range=full:out_range=tv:out_color_matrix=bt709,format=yuv420p',
    '-c:v','libx264','-preset','medium','-crf','16','-threads','2','-g','120',
    '-color_primaries','bt709','-color_trc','bt709','-colorspace','bt709','-movflags','+faststart',partial],{stdio:['pipe','ignore','pipe']});
  let errors='';ff.stderr.on('data',d=>{errors=(errors+d).slice(-6000);});
  const done=new Promise((resolve,reject)=>{ff.on('error',reject);ff.on('close',code=>code===0?resolve():reject(new Error(`ffmpeg ${code}: ${errors}`)));});
  async function* frames() {
    for(let i=job.start;i<job.end;i++) {
      yield Buffer.from(await frame(page,i/FPS));
      if((i-job.start+1)%120===0)console.log(`frames ${i-job.start+1}/${job.end-job.start} · ${(i/FPS).toFixed(1)} s`);
    }
  }
  try {
    await Promise.all([pipeline(Readable.from(frames()),ff.stdin),done]);
    const p=probe(partial);
    if(Number(p.nb_frames)!==job.end-job.start)throw new Error(`Frame count mismatch: ${partial}`);
    await rename(partial,target);
  } catch(error) {ff.kill('SIGTERM');throw error;}
}

try {
  await Promise.all(Array.from({length:workers},async(_,worker)=>{
    // Separate browsers keep screenshot locks and animation-frame clocks independent.
    const browser=await launch();
    browsers.push(browser);
    const page=await openPage(browser,server.address().port);
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    if(worker===0)await writeFile('out/timeline.json',JSON.stringify(await page.evaluate(()=>window.__cues),null,2)+'\n');
    while(next<jobs.length) {
      const job=jobs[next++];
      await encode(page,job);
      if(errors.length)throw new Error(errors.join('\n'));
      completed++;
      console.log(`${completed}/${jobs.length} · ${(job.start/FPS).toFixed(1)}–${(job.end/FPS).toFixed(1)} s · elapsed ${Math.round((Date.now()-started)/1000)} s`);
    }
    await page.close();
  }));
  await writeFile(path.join(dest,'concat.txt'),jobs.map(j=>`file '${j.file}'`).join('\n')+'\n');
  const output=from===0&&to===DURATION*FPS?'picture':`picture-${from}-${to}`;
  execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','concat','-safe','1','-i',path.join(dest,'concat.txt'),
    '-c','copy','-movflags','+faststart',`out/${output}.partial.mp4`]);
  await rename(`out/${output}.partial.mp4`,`out/${output}.mp4`);
  const report={signature,from:from/FPS,to:to/FPS,fps:FPS,frames:to-from,chunks:jobs.length,elapsedSeconds:(Date.now()-started)/1000,source};
  await writeFile(`out/${output}.json`,JSON.stringify(report,null,2)+'\n');
  console.log(`out/${output}.mp4`);
} finally {
  for(const browser of browsers)await browser.close();
  server.close();
}
