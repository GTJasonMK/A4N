import { el, prog, E, lerp, reveal, addStyle, show } from '../core.js';
import { T, TEXT, STORY } from '../timeline.js';
import { CAMS } from '../gl/story.js';
import { makeCaption, makeCursor, flashAmount } from '../ui.js';

export default {
  from:T.drop,
  to:T.snapshot+.15,
  mount(root,ctx) {
    addStyle(`
      .media-frame { position:absolute; overflow:hidden; border:1px solid #ffe8bc55; border-radius:12px; box-shadow:0 32px 90px #000a; }
      .media-frame canvas { width:100%; height:100%; display:block; }
      .media-hero { position:absolute; inset:0; width:1920px; height:1080px; }
      .media-shade { position:absolute; inset:0; background:linear-gradient(#03080c33,transparent 40%,#03080cb0); }
      .media-kicker { color:var(--amber2); font:500 20px var(--mono); letter-spacing:.25em; }
      .media-title { font:500 66px/1.4 var(--serif); margin-top:22px; }
      .media-label { position:absolute; bottom:18px; left:24px; font:500 20px var(--sans); color:#fff4df; text-shadow:0 2px 9px #000; }
      .media-pill { position:absolute; padding:7px 15px; border:1px solid #efe1c788; border-radius:24px; font:500 18px var(--sans); color:#f6eddc; background:#161716b0; }
      .comic-panel { position:absolute; overflow:hidden; border:7px solid #ede7db; background:#ede7db; box-shadow:0 16px 40px #0008; }
      .comic-panel canvas { width:100%; height:100%; display:block; }
      .comic-bubble { position:absolute; background:#f4efe4; border:3px solid #38342f; padding:12px 22px; color:#282724; border-radius:50%; font:500 27px/1.6 var(--serif); }
      .clip-frame { position:absolute; width:470px; height:185px; padding:5px; background:#24292c; border:1px solid #78909b77; border-radius:9px; overflow:hidden; }
      .clip-frame canvas { width:100%; height:140px; object-fit:cover; }
      .clip-frame span { font:400 17px var(--mono); color:#c9d7dd; padding-left:12px; }
      .clip-playhead { position:absolute; top:791px; width:2px; height:202px; background:#ffcd81; box-shadow:0 0 12px #f6b154aa; }
      .gal-dialog { position:absolute; left:125px; top:742px; width:1670px; height:229px; border:1px solid #d9a75688;
        border-radius:14px; padding:30px 45px; background:linear-gradient(110deg,#0c1b26f5,#0b1725dd); box-shadow:0 20px 70px #000b; }
      .gal-name { font:600 26px var(--serif); color:#ffd292; margin-bottom:20px; }
      .gal-line { font:400 39px var(--serif); color:#f0e7d9; letter-spacing:.08em; }
      .gal-choice { padding:19px 25px; margin-top:18px; border:1px solid #cab38b77; background:#101c29e8; font:400 25px var(--serif); border-radius:8px; }
    `);
    this.root=root;
    this.hero=el('canvas','media-hero',root); this.hero.width=1920; this.hero.height=1080;
    this.shade=el('div','media-shade',root);
    this.opening=el('div','abs',root);
    Object.assign(this.opening.style,{left:'120px',top:'90px'});
    el('div','media-kicker',this.opening,'05　一个故事，多种形态');
    el('div','media-title',this.opening,'现在，让它被看见。');
    this.book=el('div','abs',root,`原创演示 · ${STORY.book}`);
    Object.assign(this.book.style,{left:'125px',bottom:'95px',font:'400 23px var(--serif)',color:'#dccfbe',letterSpacing:'.2em'});

    this.imageRoot=el('div','layer',root);
    this.artFrame=el('div','media-frame',this.imageRoot);
    Object.assign(this.artFrame.style,{left:'120px',top:'287px',width:'1110px',height:'625px'});
    this.art=el('canvas','',this.artFrame); this.art.width=1110; this.art.height=625;
    el('div','media-label',this.artFrame,'场景设计 · 雾港灯塔');
    this.imageCap=makeCaption(this.imageRoot,{num:'05.1',kicker:'插画与角色',title:'文字里的世界，显影了',sub:'角色、物体与场景，成为创作素材',x:120,y:83,width:1500,titleSize:57});
    this.portraitFrame=el('div','media-frame',this.imageRoot);
    Object.assign(this.portraitFrame.style,{left:'1300px',top:'287px',width:'500px',height:'625px'});
    const portrait=el('canvas','',this.portraitFrame); portrait.width=500; portrait.height=625;
    ctx.story.draw(portrait,{time:53,cam:[-.425,.012,8.2],beam:.2,exposure:2.2});
    el('div','media-label',this.portraitFrame,'人物形象 · 苏晚');

    this.comicRoot=el('div','layer',root);
    this.comicCap=makeCaption(this.comicRoot,{num:'05.2',kicker:'漫画',title:'同一场戏，换一种阅读方式',sub:'从章节到分镜，再组织成漫画',x:120,y:78,width:1520,titleSize:56});
    this.panels=[[130,290,960,632,CAMS.wide],[1117,290,670,300,CAMS.figure],[1117,617,670,305,CAMS.ship]].map(([x,y,w,h,cam],i)=>{
      const panel=el('div','comic-panel',this.comicRoot);
      Object.assign(panel.style,{left:`${x}px`,top:`${y}px`,width:`${w}px`,height:`${h}px`});
      const cv=el('canvas','',panel); cv.width=w; cv.height=h;
      ctx.story.draw(cv,{time:57+i,cam,beam:.17+i*.08,style:1,ship:.6,exposure:2.2});
      if(i!==1){
        const bubble=el('div','comic-bubble',panel,i===0?'它自己亮了。':'雾里，浮出一艘没有帆的船。');
        Object.assign(bubble.style,{left:i===0?'56px':'40px',top:i===0?'60px':'18px',maxWidth:'560px'});
      }
      return panel;
    });

    this.videoRoot=el('div','layer',root);
    this.videoCap=makeCaption(this.videoRoot,{num:'05.3',kicker:'视频',title:'让镜头，接住上一刻',sub:'规划镜头与首帧，生成片段后组装成片',x:120,y:70,width:1600,titleSize:56});
    this.screen=el('div','media-frame',this.videoRoot);
    Object.assign(this.screen.style,{left:'145px',top:'267px',width:'910px',height:'510px'});
    this.motion=el('canvas','',this.screen); this.motion.width=910; this.motion.height=510;
    this.timecode=el('div','media-label',this.screen);
    this.videoCopy=el('div','abs',this.videoRoot,'');
    Object.assign(this.videoCopy.style,{left:'1150px',top:'380px',width:'640px'});
    el('div','media-kicker',this.videoCopy,'首帧串联 · 时间线');
    el('div','media-title',this.videoCopy,'留住空间。<br>延续动作。').style.fontSize='56px';
    el('div','',this.videoCopy,'镜头规划 → 生成 → 检查 → 合成').style.cssText='margin-top:28px;font:400 23px var(--sans);color:#c5c5ba';
    this.clips=[CAMS.lighthouse,CAMS.wide,CAMS.ship].map((cam,i)=>{
      const n=el('div','clip-frame',this.videoRoot); Object.assign(n.style,{left:`${145+i*548}px`,top:'803px'});
      const cv=el('canvas','',n); cv.width=460; cv.height=140;
      ctx.story.draw(cv,{time:61+i,cam,beam:.2+i*.1,ship:.7,exposure:2});
      el('span','',n,`0${i+1}　${['灯塔','光束','雾中之船'][i]}`);
      if(i<2){const join=el('div','media-pill',this.videoRoot,'→'); Object.assign(join.style,{left:`${633+i*548}px`,top:'873px'});}
      return n;
    });
    this.playhead=el('div','clip-playhead',this.videoRoot);

    this.galRoot=el('div','layer',root);
    this.galTitle=el('div','abs',this.galRoot,'05.4　互动故事 · GALGame');
    Object.assign(this.galTitle.style,{left:'125px',top:'85px',font:'500 29px var(--sans)',color:'#f7dca8',letterSpacing:'.13em'});
    this.choices=el('div','abs',this.galRoot); Object.assign(this.choices.style,{left:'1180px',top:'359px',width:'610px'});
    this.choiceNodes=TEXT.galChoices.map(c=>el('div','gal-choice',this.choices,c));
    const dialog=el('div','gal-dialog',this.galRoot);
    el('div','gal-name',dialog,TEXT.galName);
    this.line=el('div','gal-line',dialog);
    this.export=el('div','abs',this.galRoot,'场景 · 台词 · 选择支　→　导出 WebGAL 工程');
    Object.assign(this.export.style,{right:'132px',bottom:'60px',font:'400 20px var(--sans)',color:'#e1c18e'});
    this.cursor=makeCursor(this.galRoot,[{t:69.1,x:1720,y:669},{t:70.15,x:1515,y:402},{t:71,x:1540,y:410}],[70.2]);

    ctx.cue(48,'impact',{size:1.3}); ctx.cue(48,'ocean',{dur:24,vel:.25});
    ctx.cue(48,'bell',{note:74,vel:.9});
    [52,56,60,66].forEach(t=>ctx.cue(t,'whoosh',{dur:.65}));
    [52.8,54.1].forEach(t=>ctx.cue(t,'shutter'));
    [56.15,56.55,56.95].forEach(t=>ctx.cue(t,'paper'));
    [61,62.6,64.2].forEach((t,i)=>ctx.cue(t,'land',{i}));
    ctx.cue(66.5,'bell',{note:81,vel:.4});
    ctx.cue(68.5,'fly'); ctx.cue(70.2,'click'); ctx.cue(70.5,'check',{i:3});
    ctx.cue(71.65,'swish',{dur:.45});
  },
  update(t,ctx) {
    const first=t<52, art=t>=52&&t<56, comic=t>=56&&t<60, video=t>=60&&t<66, gal=t>=66;
    show(this.hero,first||gal); show(this.shade,first||gal); show(this.opening,first); show(this.book,first);
    show(this.imageRoot,art); show(this.comicRoot,comic); show(this.videoRoot,video); show(this.galRoot,gal);
    ctx.dust=first||gal?.17:.25; ctx.bg.state.tint=[.2,.39,.55]; ctx.bg.state.amt=.7;
    ctx.flash=flashAmount(t,48,.01,.75)*.55;
    this.root.style.opacity=`${1-prog(t,71.75,72.12)}`;
    if(first||gal){
      const cam=first?[-.02+prog(t,48,52)*.018,.03,1+prog(t,48,52)*.1]:[-.07,.025,1.12];
      ctx.story.draw(this.hero,{time:t,cam,beam:.2+(t-48)*.16,ship:.45,exposure:1.9});
      reveal(this.opening,E.outCubic(prog(t,48.2,49.2)),{dy:25});
      this.book.style.opacity=`${prog(t,49.5,50.4)}`;
    }
    if(art){
      this.imageCap.update(t,52.1,56.2);
      const u=E.outCubic(prog(t,52,52.8));
      this.artFrame.style.transform=`perspective(1800px) rotateY(${(1-u)*18}deg) translateY(${(1-u)*50}px)`;
      this.artFrame.style.opacity=`${u}`;
      ctx.story.draw(this.art,{time:53,cam:CAMS.wide,beam:.25,noise:1-E.outCubic(prog(t,52.25,53.05)),ship:.5,exposure:1.9});
      reveal(this.portraitFrame,E.outCubic(prog(t,53.35,54.15)),{dy:40,blur:12});
    }
    if(comic){
      this.comicCap.update(t,56,60.2);
      this.panels.forEach((n,i)=>{
        const u=E.outCubic(prog(t,56+i*.35,56.75+i*.35));
        n.style.opacity=`${u}`;
        n.style.transform=`perspective(1600px) rotateY(${(1-u)*-22}deg) translateY(${(1-u)*50}px)`;
      });
    }
    if(video){
      this.videoCap.update(t,60,66.15);
      const u=prog(t,60.5,65.7);
      const cam=CAMS.wide.map((v,i)=>lerp(v,CAMS.ship[i],E.inOutSine(u)*.65));
      ctx.story.draw(this.motion,{time:t,cam,beam:(t-60)*.25,ship:.5,exposure:1.9});
      this.timecode.textContent=`▶　00:00:${String(Math.floor((t-60)*1.5)).padStart(2,'0')}`;
      reveal(this.videoCopy,E.outCubic(prog(t,60.3,61.1)),{dx:30,dy:0});
      this.clips.forEach((n,i)=>{n.style.borderColor=u>=i/3&&u<(i+1)/3?'#ffcd81':'#78909b77';});
      this.playhead.style.left=`${145+1566*u}px`;
    }
    if(gal){
      reveal(this.galRoot,E.outCubic(prog(t,66,66.5)),{dy:0,blur:0});
      this.line.textContent=TEXT.galLine.slice(0,Math.floor(prog(t,66.6,68.2)*TEXT.galLine.length));
      reveal(this.choices,E.outCubic(prog(t,68.5,69.1)),{dy:20});
      this.choiceNodes.forEach((n,i)=>{
        n.style.borderColor=i===0&&t>=70.2?'#f2c784':'#cab38b77';
        n.style.background=i===0&&t>=70.2?'#705c37ed':'#101c29e8';
        n.style.opacity=i===1&&t>=70.2?'.4':'1';
      });
      this.export.style.opacity=`${prog(t,70.5,71.1)}`;
      this.cursor.update(t,prog(t,69,69.25));
    }
  },
};
