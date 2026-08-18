export interface VisualStyleTemplate {
  id: string;
  name: string;
  summary: string;
  fields: {
    styleNames: string;
    tone: string;
    lighting: string;
    color: string;
    composition: string;
    camera: string;
    material: string;
    atmosphere: string;
    detail: string;
    motion: string;
    avoid: string;
  };
}

export const renderVisualStyleTemplate = (template: VisualStyleTemplate): string => {
  const { fields } = template;
  return [
    `风格模板：${template.name}`,
    `风格关键词：${fields.styleNames}`,
    `整体基调：${fields.tone}`,
    `光影：${fields.lighting}`,
    `色彩：${fields.color}`,
    `构图：${fields.composition}`,
    `镜头：${fields.camera}`,
    `材质与笔触：${fields.material}`,
    `氛围：${fields.atmosphere}`,
    `细节密度：${fields.detail}`,
    `运动表现：${fields.motion}`,
    `避免项：${fields.avoid}`,
  ].join("\n");
};

export const VISUAL_STYLE_TEMPLATES: VisualStyleTemplate[] = [
  {
    id: "cinematic-realism",
    name: "电影感写实",
    summary: "适合角色、场景、视频分镜和剧情向画面。",
    fields: {
      styleNames: "电影感写实、cinematic realism、film still、live-action movie frame、35mm cinematic photography",
      tone: "真实、克制、可拍摄，保留小说场景的戏剧张力",
      lighting: "主光方向明确，使用自然光、窗光或低位环境光，保留柔和阴影和高光层次",
      color: "低到中等饱和度，冷暖对比清楚，肤色和环境色不过度偏色",
      composition: "主体清晰，前中后景有空间纵深，构图服务情绪和叙事重点",
      camera: "35mm 或 50mm 电影镜头质感，中景到近景为主，景深自然不过度虚化",
      material: "写实皮肤、布料、金属、木石等材质，纹理细节可信",
      atmosphere: "沉浸式剧情氛围，空气透视、尘埃、雨雾等环境细节适度",
      detail: "中高细节，关键人物和道具清楚，背景不喧宾夺主",
      motion: "视频中动作连贯、重心稳定、镜头运动克制；图片中保持瞬间定格感",
      avoid: "避免过度磨皮、塑料质感、强 HDR、夸张光晕、广告棚拍感",
    },
  },
  {
    id: "anime-cel",
    name: "动画赛璐璐",
    summary: "适合轻小说、青春、幻想和角色立绘。",
    fields: {
      styleNames: "动画赛璐璐、anime cel shading、Japanese TV animation、clean line art、cel anime illustration",
      tone: "清爽、角色辨识度高，画面有动画分镜感",
      lighting: "干净的二到三段式明暗，边缘高光明确，阴影形状简洁",
      color: "明亮但不过曝，主色和辅色区分清楚，局部使用高饱和点缀",
      composition: "角色轮廓优先，姿态和表情突出，背景简化但保留地点信息",
      camera: "动画镜头语言，中近景和半身构图清晰，透视不过度夸张",
      material: "赛璐璐块面、干净线稿、少量柔和渐变，不做厚重油画笔触",
      atmosphere: "轻快、明亮，有青春感或幻想感，情绪表达直接",
      detail: "中等细节，发型、服装、眼神和关键道具必须精确",
      motion: "视频中动作有动画节奏和清晰关键帧；图片中姿态定格自然",
      avoid: "避免写实皮肤毛孔、杂乱线条、过厚涂抹、复杂噪点背景",
    },
  },
  {
    id: "kyoani-everyday-anime",
    name: "京阿尼动画风格",
    summary: "适合青春、校园、日常、治愈、轻幻想和细腻情绪场景。",
    fields: {
      styleNames: "京阿尼动画风格、Kyoto Animation inspired anime、KyoAni-style everyday anime、slice-of-life anime、delicate character acting",
      tone: "温柔、清透、日常感强，强调角色细腻情绪和生活瞬间的真实触感",
      lighting: "柔和自然光、窗边逆光或傍晚暖光，光斑和空气感轻盈，阴影边缘干净透明",
      color: "明亮通透的中高明度色彩，低到中等饱和度，肤色自然，背景色和服装色协调克制",
      composition: "角色与环境关系明确，常用平视中景、近景和安静留白，突出眼神、手部和细小动作",
      camera: "动画电影式镜头语言，轻微景深、自然透视，适合校园走廊、窗边、街道、社团教室等日常空间",
      material: "干净线稿、赛璐璐块面结合柔和渐变，头发高光、眼睛反光、制服布料和背景细节精致但不厚重",
      atmosphere: "青春、温暖、清新、带一点怀旧和心动感，情绪表达克制但可被表情和动作读出",
      detail: "中高细节，重点放在表情、发丝、手势、制服褶皱、教室道具和季节性背景",
      motion: "视频中动作流畅细腻，重视眨眼、转头、发丝摆动、衣角和手部小动作；图片中保持自然定格",
      avoid: "避免过度性感化、夸张网游光效、厚涂油画感、强烈脏噪点、面部崩坏、过度复杂装饰和复刻具体作品角色或镜头",
    },
  },
  {
    id: "fantasy-concept",
    name: "奇幻厚涂概念",
    summary: "适合史诗奇幻、玄幻、怪物、场景和道具设定。",
    fields: {
      styleNames: "奇幻厚涂概念、fantasy concept art、epic fantasy illustration、painterly concept design、high fantasy key art",
      tone: "宏大、神秘、带史诗感，强调世界观尺度和材质重量",
      lighting: "戏剧化侧光或逆光，使用火光、魔法光、天光形成强层次",
      color: "深色基底搭配金色、青蓝或宝石色高光，冷暖冲突明显",
      composition: "大形体优先，主体与环境形成尺度对比，画面重心稳定",
      camera: "广角或低机位增强规模感，角色和建筑要有清晰轮廓",
      material: "厚涂笔触，金属、皮革、岩石、符文、织物质感厚重",
      atmosphere: "神秘、庄严、危险，烟雾、尘埃、魔法粒子适度",
      detail: "高细节，重点集中在脸、武器、纹章、建筑符号和魔法源",
      motion: "视频中动作有重量和惯性；图片中强调蓄势、对峙或仪式瞬间",
      avoid: "避免廉价页游光效、满屏粒子、比例失控、装饰无叙事功能",
    },
  },
  {
    id: "watercolor-storybook",
    name: "透明水彩绘本",
    summary: "适合温情、童话、回忆、治愈和轻幻想。",
    fields: {
      styleNames: "透明水彩绘本、transparent watercolor、watercolor storybook illustration、children's book art、soft hand-painted illustration",
      tone: "柔和、手作、温暖，保留纸张呼吸感",
      lighting: "漫射自然光，阴影透明轻薄，不使用强烈硬边高光",
      color: "低饱和粉彩、淡蓝、暖黄和草木绿，色彩边缘自然晕染",
      composition: "留白充足，主体与环境关系清楚，画面节奏舒缓",
      camera: "平视或轻微俯视，镜头不侵略，适合叙事插画",
      material: "水彩纸纹、透明叠色、轻线稿，边缘有自然水痕",
      atmosphere: "安静、怀旧、治愈，强调季节、天气和情绪",
      detail: "中低细节，重点在轮廓、表情和环境意象，不堆砌纹理",
      motion: "视频中运动轻柔、像绘本翻页；图片中保持静谧瞬间",
      avoid: "避免强对比赛博色、金属锐利质感、3D 渲染感、过度锐化",
    },
  },
  {
    id: "ink-oriental",
    name: "东方水墨国风",
    summary: "适合古风、武侠、仙侠、山水场景和意境化画面。",
    fields: {
      styleNames: "东方水墨国风、Chinese ink wash painting、guofeng ink illustration、shanshui painting、xieyi brushwork",
      tone: "含蓄、留白、诗意，强调气韵和意境",
      lighting: "柔和天光或月光，明暗不硬切，以墨色浓淡表现层次",
      color: "墨色、米白、青绿、朱砂点色，整体克制不花哨",
      composition: "大量留白，山水、人物、建筑形成疏密节奏",
      camera: "卷轴式长景或平远构图，人物可小但姿态要明确",
      material: "宣纸纹理、干湿墨、飞白、淡彩晕染，笔触有书写感",
      atmosphere: "清冷、空灵、江湖感或仙气，雾气和云水自然流动",
      detail: "低到中细节，重点在轮廓、衣袂、山石树影和笔墨节奏",
      motion: "视频中动作飘逸但重心清楚；图片中表现衣袂、云雾或水波的势",
      avoid: "避免日式浮世绘混用、过度金色装饰、网游仙侠廉价特效",
    },
  },
  {
    id: "noir-thriller",
    name: "黑色电影悬疑",
    summary: "适合犯罪、悬疑、谍战、都市阴谋和压迫感场景。",
    fields: {
      styleNames: "黑色电影悬疑、film noir、neo-noir thriller、chiaroscuro lighting、noir crime cinema",
      tone: "冷峻、危险、压抑，有秘密和不信任感",
      lighting: "强烈明暗对比，百叶窗光、路灯、霓虹反射或单点硬光",
      color: "黑灰蓝为主，少量红色或钠灯黄作为危险信号",
      composition: "斜线、遮挡、框中框，人物常处在阴影或门框边缘",
      camera: "35mm 街头纪实感，低机位或侧后方视角增强窥视感",
      material: "湿润街面、烟雾、旧墙面、皮革、玻璃反射和胶片颗粒",
      atmosphere: "雨夜、烟雾、狭窄空间、城市噪声感",
      detail: "中高细节，关键线索、表情和手部动作必须清楚",
      motion: "视频中镜头缓慢推进或跟拍，保持紧张和可读性",
      avoid: "避免明亮糖果色、喜剧化夸张表情、过度干净的商业棚拍",
    },
  },
  {
    id: "cyberpunk-neon",
    name: "赛博霓虹",
    summary: "适合未来都市、科技阴谋、义体、夜景和高反差视觉。",
    fields: {
      styleNames: "赛博霓虹、cyberpunk neon、neon noir、futuristic city night、high-tech low-life sci-fi",
      tone: "高科技与低生活并置，拥挤、潮湿、危险",
      lighting: "霓虹招牌、屏幕光、背光和雨夜反射构成主要光源",
      color: "青蓝、品红、紫色和酸性绿色高对比，暗部保留层次",
      composition: "垂直城市空间、密集招牌、人物被光牌和电缆包围",
      camera: "广角街拍或低机位，适合表现都市压迫和空间纵深",
      material: "金属、玻璃、湿地面、全息屏、义体和线缆质感清晰",
      atmosphere: "潮湿、拥挤、电子噪声、夜雨和都市孤独感",
      detail: "高细节但主次明确，界面文字不要生成可读乱码",
      motion: "视频中光影闪烁克制，人物动作稳定，避免快速跳切导致失真",
      avoid: "避免满屏乱码文字、霓虹过曝、主体被招牌淹没、廉价科幻塑料感",
    },
  },
  {
    id: "retro-film",
    name: "复古胶片",
    summary: "适合年代感、回忆、现实主义、旅行和生活片段。",
    fields: {
      styleNames: "复古胶片、retro film photography、35mm film still、nostalgic analog photo、1970s-1990s color film",
      tone: "怀旧、自然、生活化，像 70-90 年代胶片照片或电影剧照",
      lighting: "自然光、窗边逆光、傍晚暖光，保留轻微曝光不均",
      color: "暖色偏黄、轻微褪色，暗部有青绿色或棕色倾向",
      composition: "生活抓拍感，主体不必过分居中，但叙事重点清楚",
      camera: "35mm 胶片镜头，中等景深，轻微颗粒和镜头边缘柔化",
      material: "胶片颗粒、旧照片质感、织物和木质细节真实",
      atmosphere: "私人记忆、时间沉淀、温柔但不甜腻",
      detail: "中等细节，保留自然瑕疵，不追求商业级锐利",
      motion: "视频中像手持胶片纪录，轻微呼吸感但不晃动失控",
      avoid: "避免过度锐化、HDR、现代广告质感、假复古滤镜过重",
    },
  },
];

export const DEFAULT_IMAGE_STYLE = renderVisualStyleTemplate(VISUAL_STYLE_TEMPLATES[0]);
export const DEFAULT_VIDEO_STYLE = renderVisualStyleTemplate(VISUAL_STYLE_TEMPLATES[0]);
