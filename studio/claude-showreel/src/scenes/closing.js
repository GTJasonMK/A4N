import { el, prog, E, reveal, addStyle, show } from '../core.js';
import { T, TEXT, USAGE_STEPS, FLOW, REPOSITORY } from '../timeline.js';
import { makeCaption, makeWindow } from '../ui.js';

export const usage = {
  from:T.usage-.1,
  to:T.outro+.1,
  mount(root,ctx) {
    addStyle(`
      .usage-nav { position:absolute; left:125px; top:257px; right:125px; display:flex; gap:13px; }
      .usage-nav span { flex:1; padding:13px 22px; border-top:2px solid #e7c89433; font:400 23px var(--sans); color:var(--ink3); }
      .usage-number { position:absolute; left:116px; top:382px; font:200 142px var(--serif); color:#d4a465; line-height:1; }
      .usage-heading { position:absolute; left:125px; top:578px; width:440px; font:500 45px var(--serif); }
      .usage-desc { position:absolute; left:125px; top:665px; width:400px; font:300 25px/1.85 var(--sans); color:var(--ink2); }
      .usage-command { padding:16px 0; font:400 24px var(--mono); border-bottom:1px solid #d9bb8e15; }
      .usage-command small { display:block; font:400 18px var(--sans); color:#a99d89; padding-top:9px; }
      .usage-note { position:absolute; left:655px; top:972px; color:#b3a38e; font:400 20px var(--sans); }
      .usage-settings-side { position:absolute; left:0; top:0; bottom:0; width:207px; background:#11100c55; border-right:1px solid #ddc09422; padding:30px 18px; }
      .usage-settings-side div { font:400 21px var(--sans); padding:13px 15px; margin-bottom:10px; border-radius:8px; }
      .usage-field { margin:22px 0; font:400 18px var(--sans); color:#b9a890; }
      .usage-field span { display:block; padding:12px 18px; border:1px solid #dec39844; border-radius:7px; color:#e4d5bd; font:400 23px var(--mono); margin-top:9px; }
      .usage-flow { position:absolute; width:318px; height:141px; background:#332a1f; border:1px solid #d4b17855; border-radius:10px; padding:18px 23px; }
      .usage-flow b { display:block; font:400 20px var(--mono); color:#c8a164; margin-bottom:13px; }
      .usage-flow span { font:400 23px var(--sans); }
    `);
    this.cap=makeCaption(root,{num:'开始',kicker:'上手路线',title:'四步，开始创作',sub:'从源码启动，到写下自己的第一章',x:120,y:77,width:1500,titleSize:62});
    const nav=el('div','usage-nav',root);
    this.nav=USAGE_STEPS.map((s,i)=>el('span','',nav,`0${i+1}　${s.title}`));
    this.number=el('div','usage-number',root);
    this.title=el('div','usage-heading',root);
    this.desc=el('div','usage-desc',root);
    this.note=el('div','usage-note',root);
    this.windows=USAGE_STEPS.map(s=>makeWindow(root,{w:1150,h:578,x:650,y:359,title:s.title}));
    this.termLines=this.windows.slice(0,2).map((win,i)=>{
      const body=el('div','',win.body); body.style.padding='15px 33px';
      return USAGE_STEPS[i].lines.map(([cmd,note])=>{
        const line=el('div','usage-command',body);
        const code=el('div','',line); code.textContent='$ '+cmd;
        if(note) el('small','',line,note);
        return line;
      });
    });
    const launch=el('div','',this.windows[1].body,'Web 端　／　桌面端');
    launch.style.cssText='margin:75px 34px 0;padding:22px 30px;border-left:3px solid #d6ab6a;font:300 38px var(--serif);color:#e3ceb0';
    const side=el('div','usage-settings-side',this.windows[2].body);
    ['LLM 服务','嵌入模型','图片生成','视频生成'].forEach((s,i)=>{
      const n=el('div','',side,s);if(i===0)n.style.cssText='color:#f2d298;background:#70512c55';
    });
    const form=el('div','abs',this.windows[2].body);Object.assign(form.style,{left:'242px',top:'0px',width:'839px'});
    [['接口地址','https://api.example.com/v1'],['访问密钥','••••••••••••••••'],['模型名称','填写服务商提供的模型 ID']].forEach(([k,v])=>{
      const f=el('div','usage-field',form,k);el('span','',f,v);
    });
    const actions=el('div','',form);actions.style.cssText='display:flex;gap:18px;margin-top:24px';
    this.buttons=['保存','测试连接','设为默认'].map((s,i)=>el('div',i===0?'btn btn-primary':'btn btn-ghost',actions,s));
    this.modelState=el('div','abs',this.windows[2].body,'');Object.assign(this.modelState.style,{left:'242px',bottom:'21px',font:'400 20px var(--sans)',color:'var(--green)'});
    const flow=this.windows[3].body;
    this.flow=FLOW.map((s,i)=>{
      const n=el('div','usage-flow',flow);Object.assign(n.style,{left:`${38+(i%3)*365}px`,top:`${42+Math.floor(i/3)*191}px`});
      el('b','',n,`0${i+1}`);el('span','',n,s);return n;
    });
    const alt=el('div','abs',flow,'也可以选择 AI 辅助创作，从灵感对话与蓝图开始。');
    Object.assign(alt.style,{left:'40px',top:'459px',font:'400 20px var(--sans)',color:'#c8b18b'});
    USAGE_STEPS.forEach((s,i)=>{ctx.cue(s.start,'whoosh',{dur:.5});ctx.cue(s.start+.55,'bell',{note:[74,77,81,86][i],vel:.45});});
    [104.35,105.3,106.15].forEach((t,i)=>{ctx.cue(t,'click');ctx.cue(t+.15,'check',{i});});
    FLOW.forEach((_,i)=>ctx.cue(108+i*.42,'land',{i,vel:.45}));
    ctx.cue(112,'reverseTo',{dur:1.2});
  },
  update(t,ctx) {
    this.cap.update(t,94.1,111.8);
    const index=Math.max(0,USAGE_STEPS.findLastIndex(s=>t>=s.start));
    const step=USAGE_STEPS[index],u=E.outCubic(prog(t,step.start,step.start+.55));
    const out=1-prog(t,111.55,112.05);
    this.nav.forEach((n,i)=>{n.style.color=i===index?'var(--amber2)':'var(--ink3)';n.style.borderColor=i<=index?'#d4a465':'#e7c89433';n.style.opacity=`${out}`;});
    this.number.textContent=`0${index+1}`;this.title.textContent=step.title;this.desc.textContent=step.desc;
    [this.number,this.title,this.desc].forEach(n=>reveal(n,u*out,{dy:14}));
    this.windows.forEach((w,i)=>{show(w.root,i===index);if(i===index)reveal(w.root,u*out,{dy:25,blur:6});});
    this.note.textContent=[
      '.env 请填写数据库地址与 JWT 密钥；命令在自己的工作目录内执行。',
      '选择一种启动方式；完整安装说明见仓库文档。',
      '嵌入模型用于检索；图片与视频服务按需配置。',
      '导入入口：已生成章节 → 导入章节；先分析，再进入写作助手。',
    ][index];
    this.note.style.opacity=`${u*out}`;
    if(index<2)this.termLines[index].forEach((n,i)=>reveal(n,E.outCubic(prog(t,step.start+.15+i*.24,step.start+.65+i*.24)),{dx:12,dy:0,blur:2}));
    if(index===2){
      this.buttons.forEach((b,i)=>b.style.boxShadow=t>=[104.35,105.3,106.15][i]?'0 0 18px #dfbd5f55':'none');
      this.modelState.textContent=t>=106.3?'已设为默认 ✓':t>=105.45?'连接测试成功 ✓':t>=104.5?'配置已保存 ✓':'';
    }
    if(index===3)this.flow.forEach((n,i)=>reveal(n,E.outCubic(prog(t,108+i*.42,108.55+i*.42)),{dy:22,blur:4}));
    ctx.bg.state.amt=.6;
  },
};

export const outro = {
  from:T.outro-.1,
  to:T.end+.01,
  mount(root,ctx) {
    this.root=root;
    this.rings=Array.from({length:3},(_,i)=>{
      const n=el('div','abs',root);Object.assign(n.style,{left:`${960-(360+i*78)/2}px`,top:`${355-(360+i*78)/2}px`,width:`${360+i*78}px`,height:`${360+i*78}px`,border:'1px solid #e8be7655',borderRadius:'50%',boxShadow:'0 0 25px #e8be7618'});return n;
    });
    this.logo=el('img','abs',root);this.logo.src='assets/logo.svg';Object.assign(this.logo.style,{left:'752px',top:'146px',width:'416px',height:'416px'});
    this.word=el('div','abs',root,'A4N');Object.assign(this.word.style,{left:'0',right:'0',top:'524px',font:'300 41px var(--sans)',letterSpacing:'.42em',paddingLeft:'.42em',textAlign:'center',color:'#d8b583'});
    this.tagline=el('div','abs',root,TEXT.tagline);Object.assign(this.tagline.style,{left:'0',right:'0',top:'645px',font:'500 67px var(--serif)',textAlign:'center',letterSpacing:'.055em'});
    this.sub=el('div','abs',root,'小说写作　·　漫画　·　视频　·　互动故事');Object.assign(this.sub.style,{left:'0',right:'0',top:'772px',font:'300 27px var(--sans)',textAlign:'center',color:'var(--ink2)'});
    this.address=el('div','abs',root,REPOSITORY);Object.assign(this.address.style,{left:'0',right:'0',top:'902px',font:'400 25px var(--mono)',textAlign:'center',color:'#e4c59a'});
    this.license=el('div','abs',root,'开源 · Apache-2.0');Object.assign(this.license.style,{left:'0',right:'0',top:'951px',font:'400 19px var(--sans)',textAlign:'center',color:'#a9957b'});
    ctx.cue(112,'impact',{size:.7});ctx.cue(113,'bell',{note:86,vel:.65});
    ctx.cue(115,'shimmer',{dur:2,vel:.5});ctx.cue(119,'bell',{note:74,vel:.3});
  },
  update(t,ctx) {
    const fade=1-E.inOutSine(prog(t,121,123));
    this.root.style.opacity=`${fade}`;
    reveal(this.logo,E.outCubic(prog(t,112,113.25)),{dy:25,scale:.1,blur:10});
    this.logo.style.filter=`drop-shadow(0 0 ${25+5*Math.sin(t*1.3)}px #dc992760)`;
    this.rings.forEach((n,i)=>{const u=prog(t,112+i*.2,114.5+i*.2);n.style.transform=`scale(${.5+u*.9})`;n.style.opacity=`${Math.sin(u*Math.PI)*.55}`;});
    reveal(this.word,E.outCubic(prog(t,112.7,113.7)),{dy:12});
    reveal(this.tagline,E.outCubic(prog(t,113.6,115.2)),{dy:35,blur:16});
    reveal(this.sub,E.outCubic(prog(t,115,116)),{dy:18});
    reveal(this.address,E.outCubic(prog(t,116.1,117)),{dy:10});
    reveal(this.license,E.outCubic(prog(t,116.7,117.5)),{dy:8});
    ctx.bg.state.amt=.75*fade;ctx.bg.state.glow=.9*fade;ctx.bg.state.fade=fade;ctx.dust=.45*fade;
  },
};
