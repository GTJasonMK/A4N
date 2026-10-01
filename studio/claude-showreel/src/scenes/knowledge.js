import { el, svgEl, prog, E, lerp, reveal, addStyle } from '../core.js';
import { T, TEXT } from '../timeline.js';
import { makeCaption, makeWindow, makeCursor } from '../ui.js';

export const retrieve = {
  from: T.retrieve - 0.2,
  to: T.review + 0.15,
  mount(root, ctx) {
    addStyle(`
      .memory-book { position:absolute; width:132px; height:238px; padding:24px 17px;
        border:1px solid #6e5030; border-left:5px solid #c88c42; border-radius:5px 14px 14px 5px;
        background:linear-gradient(135deg,#34291e,#16120e); box-shadow:14px 24px 40px #0009;
        transform-origin:center bottom; }
      .memory-book b { display:block; font:300 54px var(--serif); color:var(--amber2); }
      .memory-book small { display:block; font:400 18px var(--sans); margin-top:16px; color:var(--ink2); }
      .memory-book i { display:block; height:3px; background:#c6a77b44; margin-top:16px; }
      .memory-current { width:168px; height:260px; border-color:var(--amber); background:#40301b; }
      .memory-future { filter:grayscale(1); border-style:dashed; }
      .memory-budget { position:absolute; left:280px; top:837px; width:1190px; }
      .memory-budget h4 { font:400 23px var(--sans); color:var(--ink2); margin-bottom:20px; }
      .memory-segments { display:flex; height:46px; gap:6px; overflow:hidden; border-radius:5px; }
      .memory-segments span { display:flex; align-items:center; justify-content:center; font:500 18px var(--sans); color:#151510; transform-origin:left; }
      .review-label { font:500 20px var(--sans); color:var(--ink3); letter-spacing:.15em; }
      .review-line { font:400 33px/2 var(--serif); margin-top:30px; }
      .review-line del { color:#ffa79b; background:#a639292b; text-decoration-thickness:2px; }
      .review-line ins { color:#b8edc5; background:#39855130; text-decoration:none; }
      .review-actions { display:flex; align-items:center; gap:18px; margin-top:36px; }
      .review-toast { position:absolute; left:515px; top:849px; width:890px; padding:23px 32px;
        border:1px solid #cb9e5155; border-radius:12px; background:#2b2319; font:400 23px var(--sans); }
    `);
    this.cap = makeCaption(root, { num:'03', kicker:'自反馈检索', title:'让故事记得，已经发生的事',
      sub:'写第 8 章时，检索范围止于第 7 章', x:120, y:90, width:1550 });
    this.books = Array.from({ length:10 }, (_, i) => {
      const current = i === 7;
      const n = el('div', `memory-book${current ? ' memory-current' : i > 7 ? ' memory-future' : ''}`, root);
      n.style.left = `${i < 7 ? 120 + i * 150 : 1235 + (i - 7) * 198}px`;
      n.style.top = `${current ? 368 : 420 + Math.abs(i - 3) * 3}px`;
      el('b','',n,String(i+1).padStart(2,'0'));
      el('small','',n,current ? '正在写作' : i > 7 ? '未来章节' : ['灯塔重燃','无帆之船','潮汐日志','铜钥匙','回声','旧约定','海雾尽头'][i]);
      for (let k=0;k<3;k++) el('i','',n).style.width=`${78-k*14}%`;
      return n;
    });
    const lines = svgEl('svg',{width:1920,height:1080},root);
    lines.style.position='absolute';
    lines.style.inset='0';
    lines.style.pointerEvents='none';
    this.links = Array.from({length:7},(_,i)=>{
      const x=186+i*150, y=670+Math.abs(i-3)*3;
      const p=svgEl('path',{d:`M ${x} ${y} C ${x} 775,1319 820,1319 642`,fill:'none',stroke:i%2?'#5ec8b8':'#ffd28a','stroke-width':2.1},lines);
      p.style.filter='drop-shadow(0 0 7px #ffd28a66)';
      const dot=svgEl('circle',{r:5,fill:'#fff1d7'},lines);
      return {p,dot,length:p.getTotalLength()};
    });
    this.wall=el('div','abs',root,'后文暂不进入检索');
    Object.assign(this.wall.style,{left:'1423px',top:'320px',width:'346px',height:'460px',borderLeft:'2px dashed #aaa08f77',padding:'382px 0 0 20px',color:'var(--ink3)',font:'400 20px var(--sans)'});
    this.budget=el('div','memory-budget',root);
    el('h4','',this.budget,'将有限上下文，留给真正相关的内容');
    const segments=el('div','memory-segments',this.budget);
    this.bars=[['关键连续性','#cfaa66',3],['章节摘要','#72b5ad',4],['原文片段','#739ab8',3]].map(([label,color,flex])=>{
      const n=el('span','',segments,label); n.style.background=color; n.style.flex=flex; return n;
    });
    ctx.cue(T.retrieve,'whoosh',{dur:.8});
    this.links.forEach((_,i)=>ctx.cue(30+i*.48,'blip',{note:[62,65,69,72,74,77,81][i],vel:.6}));
    ctx.cue(34.8,'settle');
    ctx.cue(37.5,'swish',{dur:.5});
  },
  update(t,ctx) {
    ctx.bg.state.tint=[.30,.61,.67]; ctx.bg.state.tint2=[.18,.29,.43]; ctx.bg.state.amt=.65;
    this.cap.update(t,28.25,37.65);
    const fade=1-E.inCubic(prog(t,37.5,38.05));
    this.books.forEach((n,i)=>{
      const u=E.outCubic(prog(t,28.5+i*.07,29.4+i*.07));
      const pulse=Math.exp(-Math.abs(t-(30+i*.48))*6)*(i<7?1:0);
      n.style.opacity=`${u*fade*(i>7?.3:1)}`;
      n.style.transform=`perspective(900px) translateY(${(1-u)*150-pulse*18}px) rotateY(${i<7?-14:0}deg) rotateX(${(1-u)*30}deg)`;
      n.style.boxShadow=`0 0 ${pulse*55}px #ffd28a88,14px 24px 40px #0009`;
    });
    this.links.forEach(({p,dot,length},i)=>{
      const start=30+i*.48;
      const u=E.outCubic(prog(t,start,start+.75));
      p.style.strokeDasharray=`${length} ${length}`;
      p.style.strokeDashoffset=`${(1-u)*length}`;
      p.style.opacity=`${u*fade*.55}`;
      const ph=prog(t,start,start+1.1);
      const pt=p.getPointAtLength(length*ph);
      dot.setAttribute('cx',pt.x); dot.setAttribute('cy',pt.y);
      dot.style.opacity=`${Math.sin(ph*Math.PI)*fade}`;
    });
    reveal(this.wall,E.outCubic(prog(t,29.7,30.3))*fade,{dx:20,dy:0});
    reveal(this.budget,E.outCubic(prog(t,34.4,35.2))*fade,{dy:25});
    this.bars.forEach((n,i)=>n.style.transform=`scaleX(${E.outCubic(prog(t,34.7+i*.23,35.6+i*.23))})`);
  },
};

export const review = {
  from:T.review-.15,
  to:T.drop,
  mount(root,ctx) {
    this.cap=makeCaption(root,{num:'04',kicker:'作者确认',title:'决定写回什么的，始终是你',sub:'先预览差异，再确认修改',x:120,y:100,width:1560});
    const win=makeWindow(root,{w:1510,h:460,x:205,y:333,title:'写作助手 — 正文修改预览'});
    this.win=win.root;
    const body=el('div','',win.body); body.style.padding='28px 44px';
    el('div','review-label',body,`修改要求　${TEXT.diffInstruction}`);
    this.diff=el('div','review-line',body);
    this.diff.innerHTML=`${TEXT.diffKeepA}<del>${TEXT.diffDel}</del><ins>${TEXT.diffIns}</ins>${TEXT.diffKeepB}`;
    this.done=el('div','review-line',body,`${TEXT.diffKeepA}${TEXT.diffIns}${TEXT.diffKeepB}`);
    const actions=el('div','review-actions',body);
    this.apply=el('div','btn btn-primary',actions,'应用修改');
    this.reject=el('div','btn btn-ghost',actions,'拒绝修改');
    this.status=el('span','',actions,'待审批'); this.status.style.color='var(--amber2)';
    this.note=el('div','abs',root,'确认后才会写回正文，拒绝不会改动正文。');
    Object.assign(this.note.style,{left:'249px',top:'715px',font:'400 22px var(--sans)',color:'var(--ink2)'});
    this.toast=el('div','review-toast',root,'原文已变化 → 旧建议停止写回，请重新生成');
    this.cursor=makeCursor(root,[{t:40.6,x:1510,y:785},{t:42.15,x:331,y:618},{t:43,x:340,y:623},{t:43.6,x:1470,y:760}],[42.2]);
    this.dot=el('div','abs',root);
    Object.assign(this.dot.style,{left:'955px',top:'535px',width:'10px',height:'10px',borderRadius:'50%',background:'#ffe2ac',boxShadow:'0 0 30px 12px #f2ac6366'});
    ctx.cue(38.15,'swoosh',{dur:.65}); ctx.cue(39.7,'scan',{dur:1.0});
    ctx.cue(42.2,'click'); ctx.cue(42.5,'check',{i:2}); ctx.cue(44.75,'notice');
    ctx.cue(48,'reverseTo',{dur:1.5});
  },
  update(t,ctx) {
    this.cap.update(t,38.25,46.9);
    const enter=E.outCubic(prog(t,38,38.7));
    const collapse=E.inExpo(prog(t,46.85,47.9));
    this.win.style.opacity=`${enter*(1-collapse)}`;
    this.win.style.transform=`perspective(1800px) translateY(${(1-enter)*100}px) rotateX(${(1-enter)*12}deg) scale(${1-collapse*.95})`;
    const accepted=t>=42.45;
    this.diff.style.display=accepted?'none':''; this.done.style.display=accepted?'':'none';
    this.diff.style.clipPath=`inset(0 ${(1-E.outCubic(prog(t,39.25,40.4)))*100}% 0 0)`;
    this.apply.textContent=accepted?'已应用 ✓':'应用修改';
    this.apply.style.transform=`scale(${1-.08*Math.exp(-Math.abs(t-42.2)*20)})`;
    this.reject.style.opacity=accepted?'.3':'1';
    this.status.textContent=accepted?'正文已更新':'待审批';
    this.status.style.color=accepted?'var(--green)':'var(--amber2)';
    reveal(this.note,enter*(1-collapse),{dy:10});
    reveal(this.toast,E.outCubic(prog(t,44.7,45.2))*(1-collapse),{dy:24});
    this.cursor.update(t,prog(t,40.5,40.85)*(1-prog(t,43.2,43.6)));
    this.dot.style.opacity=`${Math.sin(collapse*Math.PI)}`;
    this.dot.style.transform=`scale(${1+collapse*7})`;
    ctx.bg.state.glow=.7+collapse*1.5;
    ctx.dust=.3*(1-collapse);
  },
};
