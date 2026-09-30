import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import puppeteer from 'puppeteer-core';
import { serve } from './browser.mjs';
import { CHAPTERS, DURATION } from '../src/timeline.js';

const server=await serve();
let browser;
try {
  // The animation renderer forces OpenGL past Chrome's GPU blocklist; playback uses normal Chrome defaults.
  browser=await puppeteer.launch({channel:'chrome',headless:true,args:['--no-sandbox']});
  const page=await browser.newPage();
  await page.setViewport({width:1440,height:1000,deviceScaleFactor:1});
  const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('response',r=>{if(r.status()>=400)errors.push(`${r.status()} ${r.url()}`);});
  await page.goto(`http://127.0.0.1:${server.address().port}/watch.html`);
  await page.waitForFunction(()=>document.querySelector('video').readyState>=2,{timeout:30000});
  const meta=await page.$eval('video',v=>({width:v.videoWidth,height:v.videoHeight,duration:v.duration}));
  assert.equal(meta.width,1920);assert.equal(meta.height,1080);assert(Math.abs(meta.duration-DURATION)<.05);
  await page.$eval('video',async v=>{v.muted=true;await v.play();});
  await page.waitForFunction(()=>{const v=document.querySelector('video');return v.currentTime>.5||v.error;},{timeout:10000});
  const decoded=await page.$eval('video',v=>({time:v.currentTime,error:v.error?.message??null,
    frames:v.getVideoPlaybackQuality().totalVideoFrames,audioBytes:v.webkitAudioDecodedByteCount}));
  assert.equal(decoded.error,null);assert(decoded.time>.5&&decoded.frames>0&&decoded.audioBytes>0);
  await page.$eval('video',v=>v.pause());
  const buttons=await page.$$('#chapters button');
  assert.equal(buttons.length,CHAPTERS.length);
  const seeks=[];
  for(const i of [1,4,6,8,10,12,16,18]) {
    await buttons[i].click();
    const expected=CHAPTERS[i].start+.08;
    await page.waitForFunction(t=>{const v=document.querySelector('video');return !v.seeking&&v.readyState>=2&&Math.abs(v.currentTime-t)<.1;},{timeout:15000},expected);
    seeks.push({title:CHAPTERS[i].title,time:await page.$eval('video',v=>v.currentTime)});
  }
  await buttons[6].click();
  await page.waitForFunction(()=>{const v=document.querySelector('video');return !v.seeking&&v.currentTime>48&&v.currentTime<49;});
  await page.screenshot({path:'out/review/playback-desktop.png'});
  await page.setViewport({width:390,height:844,deviceScaleFactor:1});
  await page.screenshot({path:'out/review/playback-mobile.png',fullPage:true});
  const narrow=await page.evaluate(()=>({
    viewport:innerWidth,scroll:document.documentElement.scrollWidth,
    videoWidth:document.querySelector('video').getBoundingClientRect().width,
    visibleError:!document.querySelector('#error').hidden,
  }));
  assert(narrow.scroll<=narrow.viewport);assert(narrow.videoWidth>300);assert(!narrow.visibleError);
  await buttons[14].focus();
  await page.keyboard.press('Enter');
  await page.waitForFunction(()=>{const v=document.querySelector('video');return !v.seeking&&v.currentTime>=94&&v.currentTime<95;});
  const keyboard=await page.evaluate(()=>({focused:document.activeElement.textContent,time:document.querySelector('video').currentTime}));
  assert.equal(keyboard.focused,CHAPTERS[14].title);
  assert.deepEqual(errors,[]);
  const report={metadata:meta,playbackAdvanced:true,decoded,seeks,narrow,keyboard,errors};
  await writeFile('out/review/playback.json',JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report));
} finally {
  if(browser)await browser.close();
  server.close();
}
