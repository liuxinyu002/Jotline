---
name: 项目随手记 Jotline · Cool Mist
description: 冷调灰蓝奶油 · 晨雾任务应用设计系统（色轴 210°，明度带 73–82%）
colors:
  mist-blue-cream: "#E4E9EE"
  deep-mist-blue: "#D6DDE4"
  fog-white: "#FBFCFE"
  cold-mist-ink: "#16202B"
  ink-60: "#16202B9E"
  ink-40: "#16202B66"
  ink-12: "#16202B1F"
  ink-08: "#16202B14"
  moonlight-champagne: "#E8D8AB"
  glacier-blue: "#ADCBE4"
  lake-sage: "#A9CFC5"
  mist-rose: "#DFAEC3"
  bluebell-lilac: "#B9C0E8"
  mist-grey-blue: "#A9BCCD"
  success-green: "#34C759"
  danger-red: "#E5484D"
typography:
  display:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "64px"
    fontWeight: 900
    lineHeight: 1
    letterSpacing: "-0.05em"
  headline:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "40px"
    fontWeight: 800
    lineHeight: 1.05
    letterSpacing: "-0.03em"
  title:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "28px"
    fontWeight: 800
    lineHeight: 1.3
    letterSpacing: "-0.02em"
  subtitle:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "21px"
    fontWeight: 700
    lineHeight: 1.4
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: "normal"
  label:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "0.12em"
  caption:
    fontFamily: "Inter, 'Noto Sans SC', sans-serif"
    fontSize: "11px"
    fontWeight: 800
    lineHeight: 1.4
    letterSpacing: "0.14em"
  mono:
    fontFamily: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "normal"
rounded:
  sm: "10px"
  md: "16px"
  lg: "22px"
  xl: "28px"
  full: "999px"
spacing:
  sp-1: "4px"
  sp-2: "8px"
  sp-3: "12px"
  sp-4: "16px"
  sp-5: "20px"
  sp-6: "24px"
  sp-8: "32px"
  sp-10: "40px"
components:
  button-primary:
    backgroundColor: "{colors.cold-mist-ink}"
    textColor: "#FFFFFF"
    rounded: "{rounded.full}"
    height: "44px"
    padding: "0 22px"
    typography:
      fontFamily: "Inter, 'Noto Sans SC', sans-serif"
      fontSize: "15px"
      fontWeight: 700
  button-primary-hover:
    backgroundColor: "#0D141C"
    textColor: "#FFFFFF"
    rounded: "{rounded.full}"
    height: "44px"
    padding: "0 22px"
  button-secondary:
    backgroundColor: "{colors.fog-white}"
    textColor: "{colors.cold-mist-ink}"
    rounded: "{rounded.full}"
    height: "44px"
    padding: "0 22px"
    typography:
      fontFamily: "Inter, 'Noto Sans SC', sans-serif"
      fontSize: "15px"
      fontWeight: 700
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink-60}"
    rounded: "{rounded.full}"
    height: "44px"
    padding: "0 22px"
  button-champagne:
    backgroundColor: "{colors.moonlight-champagne}"
    textColor: "{colors.cold-mist-ink}"
    rounded: "{rounded.full}"
    height: "44px"
    padding: "0 22px"
  input:
    backgroundColor: "{colors.fog-white}"
    textColor: "{colors.cold-mist-ink}"
    rounded: "{rounded.md}"
    height: "46px"
    padding: "12px 16px"
  tag-dark:
    backgroundColor: "{colors.cold-mist-ink}"
    textColor: "#FFFFFF"
    rounded: "{rounded.full}"
    padding: "6px 13px"
    typography:
      fontFamily: "Inter, 'Noto Sans SC', sans-serif"
      fontSize: "12px"
      fontWeight: 700
  tag-light:
    backgroundColor: "{colors.fog-white}"
    textColor: "{colors.cold-mist-ink}"
    rounded: "{rounded.full}"
    padding: "6px 13px"
  task-card-highlight:
    backgroundColor: "{colors.moonlight-champagne}"
    textColor: "{colors.cold-mist-ink}"
    rounded: "{rounded.lg}"
    padding: "20px"
  segmented-active:
    backgroundColor: "{colors.cold-mist-ink}"
    textColor: "#FFFFFF"
    rounded: "{rounded.full}"
    padding: "8px 20px"
---

# Design System: 项目随手记 Jotline · Cool Mist

## 1. Overview

**Creative North Star: 「晨雾工作台」**

一套冷调灰蓝奶油设计系统，由暖调「奶油莫兰迪」绕冷轴 210° 平移推导而来：雾蓝米底 × 冷雾黑胶囊 × 六色冰川马卡龙 × 22px 大圆角 × 粗黑数字。整体气质是清晨薄雾里的任务台：低饱和、安静、不抢注意力；需要确定感时，由超大粗黑数字（时间、日期、计数）提供「一眼可知」的锚点。信息密度中等偏松，层级由字重与字号驱动，而非颜色。

这套系统的应用载体是「项目随手记」（Jotline）的桌面端 UI：随手浮窗（记 / 查 / 看）、操作卡片、待办中心、时间线。它明确拒绝 PRODUCT.md 中的全部 anti-references：高饱和 SaaS 蓝渐变 hero、深色霓虹 dashboard、厚重拟物、密集小字企业后台、全屏糖果色铺满。

动效纪律：统一缓动 `--ease-out: cubic-bezier(.22,1,.36,1)`；微交互 `.18s`（--dur-1）、浮层 / 开关 `.32s`（--dur-2）；只动 transform 与 opacity，不动布局属性；`prefers-reduced-motion` 下全量关停。标志性微交互：图标按钮与 FAB 悬停微转 90°，卡片悬停上浮 2–4px 并升一档阴影。

布局：4pt 间距体系；内容区 `min(1140px, 92%)` 居中；散文类正文行长 ≤ 600px（约 65–75ch）。

**Key Characteristics:**

- 一切交互元素皆为「胶囊」（--r-full 999px）
- 冷雾黑 #16202B 是唯一的操作色；冰川马卡龙只承载内容分类
- 月光香槟 #E8D8AB 是全套色板唯一暖色，只做小面积点睛
- 阴影与描边全部从冷雾黑同源派生，永不用纯黑
- 大圆角体系（10 / 16 / 22 / 28 / full），22px 是卡片标准圆角

## 2. Colors: 冰川马卡龙色板

冷轴 210° 低饱和底承载冷雾黑与六色冰川马卡龙；颜色留给内容、黑色留给操作，香槟色只做极小面积点睛。

### Primary

- **冷雾黑 Cold Mist Ink** (#16202B, hsl 211 32% 13%): 全系统真正的「主色」——主按钮、激活态、标签胶囊、分段控制器选中、FAB、任务卡大数字。一切「可以点」的东西都是它。
- **雾蓝米 Mist Blue Cream** (#E4E9EE, hsl 210 23% 91%): 页面背景。比纯灰偏蓝 10°，是整个冷调气质的底色。
- **雾白 Fog White** (#FBFCFE): 卡片与输入框表面。#FFFFFF 只允许作为冷雾黑胶囊上的文字出现（源自设计源文件），其余一切「白」用雾白。

### Secondary

- **冰川蓝 Glacier Blue** (#ADCBE4): 主冷色，任务色板之首。工作 / 设计类内容、演示强调、滑块与圆角示例的示范色。

### Tertiary

- **月光香槟 Moonlight Champagne** (#E8D8AB): 唯一暖锚点。重点任务卡背景、「开始专注」强调按钮、小圆点点睛（logo 圆点、标签胶囊圆点）、文字选区背景。
- **湖水青 Lake Sage** (#A9CFC5): 阅读 / 放松类内容；任务行与详情头图。
- **雾玫粉 Mist Rose** (#DFAEC3): 健康类内容。
- **蓝铃紫 Bluebell Lilac** (#B9C0E8): 聚会 / 社交类内容；日历事件色。
- **雾灰蓝 Mist Grey Blue** (#A9BCCD): 杂务 / 中性内容，任务色板中最收敛的一档。

### Neutral

- **深雾蓝 Deep Mist Blue** (#D6DDE4): 深一层的背景 / 分层。
- **次级文字 ink-60** (#16202B9E): 次级文字、描述。
- **辅助文字 ink-40** (#16202B66): 占位符、角标、说明文字。
- **描边 ink-12** (#16202B1F): 输入框描边、次要按钮描边。
- **弱分隔 ink-08** (#16202B14): 卡片描边、分隔线、悬停底色。

### 功能色（跨主题恒定）

- **成功绿** (#34C759): 开关 ON、完成确认。**危险红** (#E5484D): 删除按钮。两者不随主题漂移，是全部色板中仅有的两个高饱和色。

### Named Rules

**颜色留给内容，黑色留给操作。** 冰川色板只回答「这条内容是什么 / 属于哪类」；一切可交互元素——按钮、选中、激活、主操作——用冷雾黑承载。禁止用任务色做主操作按钮，禁止用黑色做内容分类。

**唯一暖锚点 Rule。** 月光香槟是全套色板唯一的暖色，只做小面积点睛（小圆点、单张重点卡、单一强调按钮）。禁止引入第二个暖色；全冷极简变体下可将其替换为蓝铃紫 #B9C0E8（其余变量不动）。

**明度带锁死 Rule。** 六色任务色明度带锁在 73–82%（OKLCH 78–89%），保证同屏并置时明度均匀、只有色相在说话。禁止引入更亮或更暗的同类色破坏这条带。

## 3. Typography

**Display / 数字字体：** Inter（900 / 800 / 700 / 600）
**正文字体：** Inter + Noto Sans SC（中文兜底，可用字重 400 / 500 / 700 / 900）
**等宽数字：** JetBrains Mono（500）

**Character:** 数字是这套字体的灵魂：Inter 800/900 紧字距呈现「晨雾海报感」的大号时间与日期；中文由 Noto Sans SC 兜底，标题重字重、正文常规。层级完全由字重与字号驱动，几乎不用颜色。

### Hierarchy

- **Display** (900, 64px / `--text-display`, -0.05em, 行高 1): 超大日期数字「12.10」。仅首屏 / 英雄区，单屏最多一处。
- **Headline** (800, 40px / `--text-2xl`, -0.03em): 时间数字「10:00 AM」，重点任务卡的核心元素。
- **Title** (800, 28px / `--text-xl`): 页面 H1「今日任务」。
- **Subtitle** (700, 21px / `--text-lg`): 卡片标题「界面设计迭代」。
- **强调正文** (600, 17px / `--text-md`): 设置行、强调段落。
- **Body** (400, 15px / `--text-base`, 行高 1.6): 正文。行长 ≤ 65–75ch。
- **Label** (600, 12px / `--text-sm`, +0.12em): 标签、说明、大写周头「MON TUE WED」。
- **Caption** (800, 11px / `--text-xs`, +0.14em): 角标、demo-label 型微标题（大写）。
- **Mono** (500, 12–13px): 时间锚行、数据值、色值、代码。

### Named Rules

**粗黑数字 Rule。** 时间、日期、计数一律 Inter 800/900、-3% ~ -5% 字距、行高 ≤ 1.05。禁止用细字重、常规字距或颜色去表达同一层级的数字。

**字重驱动层级 Rule。** 同尺寸的层级差异用字重表达（700 vs 400），不用颜色。--ink-60 / --ink-40 只做「次级 / 辅助」语义，不做层级强调。

## 4. Elevation

阴影是「冷雾同源」的环境光体系：所有阴影色一律 rgba(22,32,43, ·)，与主墨同源，永不使用纯黑。静止表面基本是平的（描边 ink-08 + 雾白表面承担分层），阴影主要作为状态与层级的回应出现：悬停升档、浮层常驻、模态最深。

### Shadow Vocabulary

- **shadow-1** (`0 1px 2px rgba(22,32,43,.05), 0 4px 14px rgba(22,32,43,.06)`): 静置卡片、浮窗内小组件（天气、chip）。
- **shadow-2** (`0 2px 8px rgba(22,32,43,.08), 0 14px 34px rgba(22,32,43,.10)`): 悬停升起、弹层、FAB、toast。
- **shadow-3** (`0 6px 16px rgba(22,32,43,.12), 0 28px 60px rgba(22,32,43,.16)`): 模态、设备壳（手机 mockup）。

### Named Rules

**冷雾同源 Rule。** 任何 box-shadow 的颜色只能是 rgba(22,32,43, ·) 或由其派生。纯黑阴影（rgba(0,0,0,·)）禁止出现。

**平-静-浮 Rule。** 列表行、分段轨道静置无阴影；卡片静置至多 shadow-1；阴影升档必须由状态触发（hover / open / drag），不允许静态堆叠阴影。

## 5. Components

### Buttons（一切按钮皆为胶囊）

- **Shape:** 全圆角胶囊 (--r-full 999px)；高度三档 52 / 44 / 34px。
- **Primary:** 冷雾黑底 #16202B + 白字，内边距 0 22px（md 档）。hover: 底色加深至 #0D141C + 上浮 2px + shadow-2；active: scale(.97)。
- **Secondary:** 雾白底 + ink-12 描边 + 冷雾黑字。hover: 描边转冷雾黑 + 上浮 2px + shadow-1。
- **Ghost:** 透明底 ink-60 字。hover: ink-08 底 + 转冷雾黑。
- **强调档:** 香槟（月光香槟底冷雾黑字，重点动作「开始专注」）与冰川（冰川蓝底，筛选类动作）。hover 同样上浮 + shadow-2。
- **Danger:** 危险红 #E5484D 底白字。
- **Disabled:** opacity .4 + pointer-events none。
- **图标按钮 / FAB:** 44 / 34px 正圆。dark 变体冷雾黑底白字，hover 上浮 2px 微转 90°；FAB 58px，hover 上浮 3px 转角 90° + shadow-3。

### Tags（标签胶囊）

- **Style:** 全部胶囊形，带 5px 圆点（currentColor）。dark：冷雾黑底白字、圆点香槟色；light：雾白底 + shadow-1、圆点用对应任务色；ghost：透明底 ink-12 描边。
- **表情胶囊 pill-emoji:** 38×28px 冷雾黑胶囊承载 emoji，作卡片角标。

### Cards / Containers

- **Corner Style:** 卡片标准 22px (--r-lg)，小元素 16px (--r-md)。
- **Background:** 雾白 #FBFCFE；任务卡用冰川色板整卡填充（颜色即内容分类）。
- **Shadow Strategy:** 静置 shadow-1，hover 上浮 4px + shadow-2。
- **Border:** 1px ink-08。
- **Internal Padding:** 20px (--sp-5)；双列小卡 16px。

### 任务卡 Task Card（签名组件）

任务的默认形态，冰川色板轮换填充。结构：顶部（tag-dark 任务名 + 头像组）→ 大号时间数字（40px/800，重点卡 64px）→ 底部（开始时间说明 + 表情胶囊）。重点任务用香槟色 + 放大数字；列表行形态 task-row 带 36px check-circle，完成态打勾胶囊转冷雾黑 + 文字删除线 + opacity .55。

### Inputs / Fields

- **Style:** 雾白底、1.5px ink-12 描边、16px 圆角、12×16px 内边距。
- **Focus:** 描边转冷雾黑 + `0 0 0 3px rgba(22,32,43,.08)` 外圈。
- **Placeholder:** ink-40。带图标的输入框左侧留 42px。
- **开关 Toggle:** 46×28px 胶囊轨道，ON = 成功绿 #34C759，滑块 22px 白圆带投影。
- **分段控制器 Segmented:** rgba(22,32,43,.07) 胶囊轨道 + 4px 内边距；选中项为冷雾黑胶囊白字。这是浮窗「今日 / 日历」导航的核心形态。

### Navigation

顶栏 sticky、雾蓝米 82% 透明度 + 14px backdrop blur、1px ink-08 底描边。链接为 13px/600 ink-60 胶囊，hover 转 ink-08 底，active 反白为冷雾黑胶囊。logo 为冷雾黑胶囊 + 7px 香槟圆点。

### Calendar（日历）

7 列网格、3px 间隙；日期格为等宽字体 13px/600 正圆；事件日用任务色整圆填充；选中日双层描边（`0 0 0 2px 雾白, 0 0 0 4px 冷雾黑`）+ scale(1.05)。日程面板为蓝铃紫大圆角容器，内嵌雾白事件行。

## 6. Do's and Don'ts

### Do:

- **Do** 让一切交互元素保持胶囊形（--r-full），高度落在 52 / 44 / 34px 三档。
- **Do** 用冰川色板整卡填充表达内容分类，主操作一律冷雾黑胶囊。
- **Do** 时间 / 日期 / 计数用 Inter 800/900 + -3%~-5% 字距（Headline / Display 档）。
- **Do** 所有阴影使用 rgba(22,32,43,·)，缓动统一 `cubic-bezier(.22,1,.36,1)`，微交互 .18s / 浮层 .32s。
- **Do** 动效只传达状态：hover 上浮 2–4px、图标按钮微转 90°、开关滑块位移；`prefers-reduced-motion` 下全部关停。
- **Do** 功能色恒定 #34C759 / #E5484D，不随主题漂移；通道标注（☁ / 🔒）如实呈现。

### Don't:

- **Don't** 使用高饱和 SaaS 蓝 + 渐变 hero 的「AI 工具」套路感，或深色霓虹 dashboard（加密货币风）——PRODUCT.md anti-references 原文。
- **Don't** 做成密集小字 + 锐利小圆角的企业后台，或全屏高饱和粉彩铺满的糖果感。
- **Don't** 使用 border-left / border-right > 1px 彩色侧条（侧条纹强调）；用整卡填充、圆点或字重替代。
- **Don't** 渐变文字（background-clip: text + gradient）；强调用字重或字号。
- **Don't** 玻璃拟态作为默认装饰；唯一允许的 blur 是顶栏 14px 功能性 backdrop。
- **Don't** 使用「大数字 + 小标签 + 渐变强调」的 hero-metric 模板；任务卡的时间数字必须绑定真实任务上下文。
- **Don't** 排布同尺寸卡片网格（同尺寸 icon + 标题 + 文案无限重复）；用任务卡 / 小卡 / 行卡三种形态混排。
- **Don't** 引入第二个暖色；月光香槟是唯一暖锚点，且只做小面积点睛。
- **Don't** 使用纯黑 #000（阴影、描边、文字）；一切深色由冷雾黑 #16202B 派生。#FFF 仅作为冷雾黑胶囊上的文字。
- **Don't** 动画布局属性（width / top / margin）；只动 transform 与 opacity。
- **Don't** 默认弹 modal 承载本可内联 / 渐进呈现的内容（深色手机壳 mockup 与模态除外）。
- **Don't** 让完成 / 危险状态只靠颜色区分；始终带图标与文字冗余。
