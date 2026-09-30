import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { serve, launch, openPage, frame } from './browser.mjs';
import { DURATION, FPS, CHAPTERS, USAGE_STEPS } from '../src/timeline.js';

await mkdir('out/review', { recursive:true });
assert.equal(CHAPTERS[0].start, 0);
assert.equal(CHAPTERS.at(-1).end, DURATION);
for (let i=0;i<CHAPTERS.length;i++) {
  assert(CHAPTERS[i].end>CHAPTERS[i].start);
  if(i) assert.equal(CHAPTERS[i].start,CHAPTERS[i-1].end);
}
assert(USAGE_STEPS.every(s=>s.title&&s.desc));
const server=await serve();
let browser;
try {
  browser=await launch();
  const page=await openPage(browser,server.address().port);
  const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  const data=await page.evaluate(()=>window.__cues);
  assert.equal(data.FPS,FPS);
  assert(data.cues.length>100);
  for (const cue of data.cues) {
    assert(Number.isFinite(cue.t)&&cue.t>=0&&cue.t<DURATION,JSON.stringify(cue));
    if(cue.dur!==undefined) assert(cue.dur>0&&Number.isFinite(cue.dur));
  }
  await writeFile('out/timeline.json',JSON.stringify(data,null,2)+'\n');
  const ranges=await page.evaluate(()=>window.__sceneRanges);
  for(let i=0;i<DURATION*FPS;i++) assert(ranges.some(r=>i/FPS>=r.from&&i/FPS<r.to),`No scene at frame ${i}`);
  const invalid=await page.evaluate(()=>[-1,124,NaN,Infinity].every(t=>{
    try {window.__seek(t);return false;} catch(e) {return e instanceof RangeError;}
  }));
  assert(invalid);
  for(const t of CHAPTERS.flatMap(c=>[c.start+.05,(c.start+c.end)/2,c.end-.05])) {
    const bad=await page.evaluate(t=>{
      window.__seek(t);
      return [...document.querySelectorAll('[style]')].filter(n=>/NaN|Infinity/.test(n.getAttribute('style'))).map(n=>n.className);
    },t);
    assert.deepEqual(bad,[],`Invalid style at ${t}`);
  }
  const times=[6.8,24.8,33.8,45.5,50,54.7,58.4,69.2,78.6,85.5,92.7,97,106.7,119];
  const images=new Map();
  const states=new Map();
  const state=()=>page.evaluate(()=>[...document.querySelectorAll('#scenes *')].filter(n=>n.checkVisibility({checkOpacity:true,checkVisibilityCSS:true})).map(n=>({
    tag:n.tagName,attributes:[...n.attributes].map(a=>[a.name,a.value]),text:n.childElementCount?null:n.textContent,
  })));
  for(const t of times) {
    images.set(t,Buffer.from(await frame(page,t,`out/review/check-${t}-forward.png`)));
    states.set(t,await state());
  }
  const rasterDifferences=[];
  for(const t of [...times].reverse()) {
    const current=Buffer.from(await frame(page,t,`out/review/check-${t}-reverse.png`));
    assert.deepEqual(await state(),states.get(t),`Scene state depends on seek order: ${t}`);
    if(!current.equals(images.get(t))) {
      const diff=await page.evaluate(async sources=>{
        const pixels=await Promise.all(sources.map(async s=>{
          const bitmap=await createImageBitmap(await(await fetch('data:image/png;base64,'+s)).blob());
          const canvas=new OffscreenCanvas(bitmap.width,bitmap.height),g=canvas.getContext('2d');
          g.drawImage(bitmap,0,0);return g.getImageData(0,0,bitmap.width,bitmap.height).data;
        }));
        let max=0,sum=0,edgePixels=0;
        for(let i=0;i<pixels[0].length;i+=4){let large=false;for(let c=0;c<3;c++){
          const d=Math.abs(pixels[0][i+c]-pixels[1][i+c]);max=Math.max(max,d);sum+=d;large ||= d>8;
        }if(large)edgePixels++;}
        return {max,mean:sum/pixels[0].length,edgePixels};
      },[images.get(t).toString('base64'),current.toString('base64')]);
      // GPU color quantization and subpixel antialiasing vary with raster-cache reuse.
      // DOM/SVG state is exact; at most 0.005% sparse edge pixels may differ above 8/255.
      assert(diff.max<=2||(diff.edgePixels<=104&&diff.mean<.04),`Visible frame depends on seek order at ${t}: ${JSON.stringify(diff)}`);
      rasterDifferences.push({t,...diff});
    }
  }
  assert.deepEqual(errors,[]);
  const origin=`http://127.0.0.1:${server.address().port}`;
  assert.equal((await fetch(origin+'/%2e%2e%2fpackage.json')).status,403);
  assert.equal((await fetch(origin+'/%ZZ')).status,400);
  const partial=await fetch(origin+'/assets/logo.svg',{headers:{Range:'bytes=0-15'}});
  assert.equal(partial.status,206);assert.equal((await partial.arrayBuffer()).byteLength,16);
  const report={framesCovered:DURATION*FPS,chapters:CHAPTERS.length,cues:data.cues.length,
    deterministicFrames:times,rasterDifferences,sceneBoundaryChecks:CHAPTERS.length*3,errors};
  await writeFile('out/review/source-check.json',JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report));
} finally {
  if(browser) await browser.close();
  server.close();
}
