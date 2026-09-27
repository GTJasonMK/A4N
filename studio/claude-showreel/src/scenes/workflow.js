import { el, svgEl, prog, E, reveal, addStyle } from '../core.js';
import { T, TEXT, STORY } from '../timeline.js';
import { makeCaption, makeWindow, makeCursor } from '../ui.js';

export const snapshot = {
  from:T.snapshot-.15,
  to:T.engine+.1,
  mount(root,ctx) {
    addStyle(`
      .source-paper { position:absolute; left:175px; top:344px; width:448px; height:376px; padding:30px 34px;
        background:linear-gradient(145deg,#ede0c8,#cabb9f); color:#463827; border-radius:6px 18px 18px 6px; border-left:7px solid #947952; box-shadow:0 26px 65px #0007; }
      .source-paper h3 { font:600 32px var(--serif); margin-bottom:20px; }
      .source-paper p { font:400 23px/1.9 var(--serif); }
      .source-version { display:inline-block; padding:5px 14px; border:1px solid #69543477; border-radius:5px; margin-bottom:24px; font:600 21px var(--mono); }
      .source-stamp { position:absolute; left:790px; top:420px; width:310px; height:180px; padding-top:30px;
        border:2px solid #deb56b; border-radius:12px; color:#e4c18b; text-align:center; box-shadow:0 0 60px #cc954425; }
      .source-stamp b { display:block; font:500 35px var(--serif); margin-bottom:14px; }
      .source-stamp span { font:400 21px var(--sans); }
      .source-derived { position:absolute; left:1290px; width:430px; height:104px; padding:25px 30px; border:1px solid #f3d29e3b;
        border-radius:12px; background:linear-gradient(120deg,#323a3e,#1a2025); font:500 27px var(--serif); }
      .source-derived small { font:400 18px var(--sans); color:#afbfca; float:right; margin-top:6px; }
      .source-decisions { position:absolute; left:240px; top:809px; width:1440px; padding:20px 28px; border:1px solid #eacb9740; border-radius:12px; background:#191916; }
      .source-row { display:flex; align-items:center; justify-content:space-between; height:64px; font:400 24px var(--sans); }
      .source-row .btn { min-width:210px; padding:10px 18px; font-size:20px; }
      .engine-core { position:absolute; left:850px; top:354px; width:220px; height:220px; border:1px solid #ffd085a0; border-radius:50%;
        display:flex; flex-direction:column; align-items:center; justify-content:center; background:radial-gradient(#8e622944,#302011bb); box-shadow:0 0 80px #eea03540,inset 0 0 35px #c6983330; }
      .engine-core b { font:300 53px var(--sans); letter-spacing:.12em; }
      .engine-core small { font:400 20px var(--sans); color:#dcbd85; margin-top:12px; }
      .mini-app { padding:24px; font:400 20px var(--serif); color:#e0d3c2; }
      .mini-app i { display:block; height:5px; margin-top:25px; background:#c2ac8933; border-radius:3px; }
      .workflow-node { position:absolute; top:432px; width:288px; height:164px; border:1px solid #79665188; border-radius:12px;
        background:#29221b; padding:24px; text-align:center; box-shadow:0 18px 36px #0005; }
      .workflow-node b { display:block; font:500 28px var(--sans); margin-bottom:19px; }
      .workflow-node span { font:400 20px var(--sans); color:var(--ink3); }
      .workflow-status { position:absolute; left:120px; top:660px; font:400 26px var(--sans); }
    `);
    this.cap=makeCaption(root,{num:'06',kicker:'来源快照',title:'原稿向前，派生作品有迹可循',sub:'绑定创作时的来源；更新前先比较差异',x:120,y:88,width:1650,titleSize:56});
    this.paper=el('div','source-paper',root);
    this.version=el('div','source-version',this.paper,'原稿 v3');
    el('h3','',this.paper,STORY.book);
    el('p','',this.paper,'雾港的灯塔，<br>今夜重新亮起。');
    this.changed=el('p','',this.paper,'海风，吹动了苏晚的围巾。'); this.changed.style.cssText='margin-top:18px;color:#6b572e';
    this.stamp=el('div','source-stamp',root,'<b>来源快照</b><span>创作依据 · v3</span>');
    this.derived=['漫画项目','视频项目','互动故事'].map((title,i)=>{
      const n=el('div','source-derived',root,`${title}<small>基于 v3</small>`); n.style.top=`${344+i*140}px`; return n;
    });
    const svg=svgEl('svg',{width:1920,height:1080},root); svg.style.cssText='position:absolute;inset:0;pointer-events:none';
    this.paths=[svgEl('path',{d:'M623 510 H788',stroke:'#e7b86e','stroke-width':2,fill:'none'},svg),
      ...this.derived.map((_,i)=>svgEl('path',{d:`M1102 510 C1190 510,1190 ${396+i*140},1288 ${396+i*140}`,stroke:'#bb9e6b','stroke-width':2,fill:'none'},svg))];
    this.decisions=el('div','source-decisions',root);
    this.rows=[['场景描述 · 来源更新','应用安全更新'],['镜头文案 · 本地有修改','保留当前']].map(([label,action])=>{
      const n=el('div','source-row',this.decisions); const l=el('span','',n,label); const b=el('div','btn btn-ghost',n,action); return {n,l,b};
    });
    this.cursor=makeCursor(root,[{t:76,x:1350,y:1000},{t:76.65,x:1536,y:862},{t:77.2,x:1536,y:862},{t:78.1,x:1536,y:924},{t:78.8,x:1560,y:932}],[76.7,78.15]);
    ctx.cue(72,'whoosh',{dur:.55}); ctx.cue(73.1,'stamp');
    ctx.cue(74.5,'paper'); ctx.cue(76.7,'click'); ctx.cue(77,'check',{i:1});
    ctx.cue(78.15,'click'); ctx.cue(78.4,'check',{i:2});
    ctx.cue(79.55,'swish',{dur:.5});
  },
  update(t,ctx) {
    this.cap.update(t,72.2,79.7);
    const out=1-prog(t,79.55,80.05);
    reveal(this.paper,E.outCubic(prog(t,72,72.6))*out,{dy:45});
    reveal(this.stamp,E.outBack(prog(t,72.9,73.45))*out,{dy:0,scale:.35,blur:4});
    this.version.textContent=t>=74.5?'原稿 v4':'原稿 v3';
    this.changed.style.opacity=`${prog(t,74.5,75.05)}`;
    this.derived.forEach((n,i)=>reveal(n,E.outCubic(prog(t,73.2+i*.17,73.9+i*.17))*out,{dx:40,dy:0}));
    this.paths.forEach((p,i)=>{const length=p.getTotalLength();p.style.strokeDasharray=`${length} ${length}`;p.style.strokeDashoffset=`${length*(1-prog(t,72.6+i*.25,73.2+i*.25))}`;p.style.opacity=`${out}`;});
    reveal(this.decisions,E.outCubic(prog(t,75.4,76))*out,{dy:24});
    this.rows.forEach(({l,b},i)=>{
      const done=t>=[77,78.4][i];
      l.textContent=i===0?'场景描述 · 来源更新':'镜头文案 · 本地有修改';
      b.textContent=done?(i===0?'已应用 ✓':'已保留 ✓'):(i===0?'应用安全更新':'保留当前');
      b.style.color=done?'var(--green)':i===0?'var(--amber2)':'var(--ink2)';
    });
    this.cursor.update(t,prog(t,75.95,76.2)*(1-prog(t,78.7,79.1)));
    ctx.bg.state.amt=.6;
  },
};

export const engine = {
  from:T.engine-.1,
  to:T.resume+.1,
  mount(root,ctx) {
    this.cap=makeCaption(root,{num:'07',kicker:'三端共享',title:'换一个入口，还是同一套创作',sub:'桌面、网页与命令行，共用业务能力',x:120,y:85,width:1600,titleSize:56});
    const svg=svgEl('svg',{width:1920,height:1080},root); svg.style.cssText='position:absolute;inset:0';
    this.paths=['M665 466 H850','M1070 466 H1255','M960 574 V715'].map(d=>svgEl('path',{d,stroke:'#e3b16d','stroke-width':2,fill:'none'},svg));
    this.dots=this.paths.map(()=>svgEl('circle',{r:6,fill:'#ffe2a9'},svg));
    this.core=el('div','engine-core',root,'<b>A4N</b><small>共享命令目录</small>');
    this.windows=[['桌面 · Tauri',155],['网页 · Web',1255]].map(([title,x])=>{
      const w=makeWindow(root,{w:510,h:330,x,y:318,title});
      const body=el('div','mini-app',w.body,`<span style="color:#dfb87a">${STORY.book}</span>　/　写作台`);
      ['正文','分析','写作助手'].forEach((s,i)=>{const l=el('div','',body,s);l.style.cssText=`display:inline-block;margin:${i?'24px 0 0 32px':'24px 0 0 0'};font:400 17px var(--sans);color:#bcaa94`;});
      for(let i=0;i<3;i++) el('i','',body).style.width=`${92-i*14}%`;
      return w.root;
    });
    this.terminal=makeWindow(root,{w:1390,h:235,x:265,y:718,title:'CLI · agent-json · 返回结构示意'}).root;
    this.command=el('div','abs',this.terminal); Object.assign(this.command.style,{left:'35px',top:'78px',font:'400 29px var(--mono)',color:'#e8c081'});
    this.result=el('div','abs',this.terminal); Object.assign(this.result.style,{left:'35px',top:'140px',font:'400 25px var(--mono)',color:'#9dc8ad'});
    ctx.cue(80,'swoosh',{dur:.6}); ctx.cue(81,'bell',{note:74,vel:.6});
    [82.2,82.45,82.7,82.95,83.2,83.45,83.7,83.95,84.2].forEach((t,i)=>ctx.cue(t,'key',{i,soft:1}));
    ctx.cue(84.6,'check',{i:2}); ctx.cue(87.5,'swish',{dur:.55});
  },
  update(t,ctx) {
    this.cap.update(t,80.15,87.7);
    const out=1-prog(t,87.6,88.05);
    reveal(this.core,E.outCubic(prog(t,80.1,81))*out,{dy:0,scale:.4});
    this.core.style.boxShadow=`0 0 ${65+12*Math.sin(t*2)}px #eea03540,inset 0 0 35px #c6983330`;
    this.windows.forEach((n,i)=>reveal(n,E.outCubic(prog(t,80.3+i*.2,81.2+i*.2))*out,{dy:40,dx:i?-50:50}));
    this.paths.forEach((p,i)=>{
      const length=p.getTotalLength(),u=prog(t,80.75+i*.18,81.45+i*.18);
      p.style.strokeDasharray=`${length} ${length}`;p.style.strokeDashoffset=`${(1-u)*length}`;p.style.opacity=`${out*.65}`;
      const ph=((t-80.8+i*.3)%1.5)/1.5,point=p.getPointAtLength(length*Math.max(0,ph));
      this.dots[i].setAttribute('cx',point.x);this.dots[i].setAttribute('cy',point.y);this.dots[i].style.opacity=`${u*out*Math.max(0,Math.sin(ph*Math.PI))}`;
    });
    reveal(this.terminal,E.outCubic(prog(t,81.5,82.2))*out,{dy:30});
    this.command.textContent='$ '+TEXT.cliCall.slice(0,Math.floor(prog(t,82.2,84.3)*TEXT.cliCall.length));
    this.result.textContent=t>=84.6?TEXT.cliResult:'';
    ctx.bg.state.amt=.6;
  },
};

export const resume = {
  from:T.resume-.1,
  to:T.usage+.1,
  mount(root,ctx) {
    this.root=root;
    this.cap=makeCaption(root,{num:'08',kicker:'任务恢复',title:'中断后，从能复用的地方继续',sub:'输入一致时，复用已经完成的检查点',x:120,y:110,width:1650,titleSize:56});
    this.nodes=['解析章节','提取事实','同步档案','建立索引','完成'].map((label,i)=>{
      const n=el('div','workflow-node',root);n.style.left=`${120+i*350}px`;
      el('b','',n,label); const s=el('span','',n);
      if(i<4){const arrow=el('div','abs',root,'→');Object.assign(arrow.style,{left:`${425+i*350}px`,top:'490px',font:'300 30px var(--sans)',color:'#aa8559'});}
      return {n,s};
    });
    this.status=el('div','workflow-status',root);
    this.term=el('div','term',root);Object.assign(this.term.style,{left:'345px',top:'771px',width:'1230px',height:'112px',padding:'30px 38px'});
    this.command=el('div','',this.term);
    ctx.cue(88,'swoosh',{dur:.5});ctx.cue(89,'check',{i:0});ctx.cue(89.4,'check',{i:1});
    ctx.cue(90,'glitch');ctx.cue(91.3,'click');
    [92,92.55,93.1].forEach((t,i)=>ctx.cue(t,'check',{i:i+2}));
    ctx.cue(93.6,'whoosh',{dur:.5});
  },
  update(t,ctx) {
    this.cap.update(t,88.15,93.8);
    const out=1-prog(t,93.65,94.05);
    this.root.style.transform=t>=90&&t<90.15?`translateX(${Math.sin(t*143)*5}px)`:'none';
    this.nodes.forEach(({n,s},i)=>{
      const done=t>=[89,89.4,92,92.55,93.1][i];
      const cached=i<2&&t>=91.5;
      reveal(n,E.outCubic(prog(t,88.1+i*.07,88.7+i*.07))*out,{dy:35});
      n.style.borderColor=done?'#83c69588':i===2&&t>=90&&t<91.5?'#df826677':'#79665188';
      s.textContent=cached?'检查点复用 ✓':done?'已完成 ✓':i===2&&t>=90&&t<91.5?'已中断':'等待执行';
      s.style.color=done?'var(--green)':'var(--ink3)';
    });
    this.status.textContent=t<90?'工作流 #42 · 执行中':t<91.5?'工作流 #42 · 已中断':t<93.1?'检查输入一致性 → 继续执行':'工作流 #42 · 已完成';
    this.status.style.color=t>=90&&t<91.5?'#eda58b':t>=93.1?'var(--green)':'var(--ink2)';
    this.status.style.opacity=`${out}`;
    reveal(this.term,E.outCubic(prog(t,90.35,90.9))*out,{dy:20});
    this.command.textContent='$ '+TEXT.cliResume.slice(0,Math.floor(prog(t,90.65,91.3)*TEXT.cliResume.length));
    ctx.bg.state.tint=t>=90&&t<90.3?[.65,.25,.15]:[.65,.5,.28];ctx.bg.state.amt=.6;
  },
};
