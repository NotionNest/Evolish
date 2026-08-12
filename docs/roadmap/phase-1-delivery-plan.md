# Evolish Phase 1 交付路线图

> 状态：Accepted
> 日期：2026-08-11
> 需求来源：[阶段一 PRD](../reference/pm/phase-1-prd.md)与[功能对齐基线](../reference/pm/easydict-feature-baseline.md)
> 输入翻译细则：[文本输入翻译 PRD](../reference/pm/text-input-translation-prd.md)

## 交付规则

阶段一按依赖顺序完成全部用户可见能力，不以功能子集代替最终目标。平台验收顺序为 macOS → Windows → Linux；共享代码在每次合并时保持三端构建通过。Linux X11 执行完整验收，Wayland 依据公开能力矩阵验收。

每项功能开始编码前必须先完成一次 Pot/Easydict 使用痛点复核，明确 Evolish 采用、调整或拒绝原交互的原因。研究结论只描述行为，不复制 GPL 实现代码。

## Issue 必填证据

每个交付 Issue 必须包含：

- 对应的 PRD/功能基线 requirement ID；
- 平台范围和平台能力前置条件；
- Pot/Easydict 观察到的不适点、Evolish 行为决策与取舍；
- 可重复执行的自动化和人工验收命令；
- UI 功能的截图/录屏，OCR/provider 功能的固定 fixture 或脱敏响应样本；
- 性能、权限、隐私和可访问性影响；
- 已知限制，Wayland 功能还需记录桌面环境、compositor 和协议；
- 完成后更新功能对齐矩阵的证据链接。

## 依赖顺序

### Epic 1：可信应用基础

覆盖 `APP-01`、`APP-02`、`APP-05`、`APP-07`、`SET-10`—`SET-13`、`NFR-02`—`NFR-08`。

交付单实例核心、托盘/菜单栏、主窗口生命周期、类型化 IPC、错误与国际化、SQLite migration、凭据、权限诊断、日志脱敏、签名更新边界和三平台 CI。这是后续所有功能的强制依赖。

### Epic 2：输入查询闭环

覆盖 `CAP-01`、`CAP-02`、`CAP-08`—`CAP-10`、`QRY-01`—`QRY-05`、`ACT-01`—`ACT-06`、`ACT-09`、`ACT-10`、`ACT-12`、`ACT-13`。

交付手动输入、快捷键取词、深度链接和外部调用，共享语言/意图判断，并在主窗口完成明确提交、换行、清理、聚焦、复制、重试、语言交换和关闭行为。文本输入过程不自动发起 AI 请求。

### Epic 3：查询监督与结果模型

覆盖 `QRY-06`—`QRY-10`、`SRV-23`、`ACT-07`、`NFR-01`。

交付 `QuerySupervisor`、主 AI 优先与其他服务按需调度、完整响应原子展示、取消与过期隔离、语言能力映射、自适应统一结果结构、主要结果复制和交互延迟测量。未展开的其他服务不得产生请求。

### Epic 4：Provider 与设置系统

覆盖适用的 `SRV-04`—`SRV-10`、`SRV-22`—`SRV-25`、`APP-06`、`SET-01`—`SET-07`，并验证 `SRV-03`、`SRV-11`—`SRV-21` 的产品排除边界。

按编译期内置 provider 原生接入 OpenAI、Anthropic 和 Gemini，并交付可创建多个实例的 OpenAI-compatible 自定义服务；DeepSeek、Ollama、Groq、智谱 AI、GitHub Models 等兼容模型通过自定义 profile 使用。同步交付 provider descriptor、凭据、连通测试、主服务设置、服务排序/启停、窗口 profile、翻译模式和查询行为设置。传统翻译 provider 不提供配置或请求入口。

### Epic 5：历史与收藏基础

覆盖文本输入翻译 PRD 的 `TIT-HIS-01`—`TIT-HIS-07`、`TIT-FAV-01`—`TIT-FAV-05` 以及 `EVO-HIS-01`、`EVO-HIS-02`、`EVO-FAV-01`。

交付默认本地历史、搜索与筛选、保留期限、关闭历史、临时隐私模式、结果版本收藏和收藏筛选。收藏只引用稳定结果版本，不生成知识卡、不创建复习队列。

### Epic 6：窗口、自动取词与截图壳层

覆盖 `APP-03`、`APP-08`、`CAP-03`—`CAP-07`、`OCR-01`、`SET-08`、`ACT-11`，并验证 `APP-04` 的产品排除边界。

交付统一迷你窗口、多显示器/DPI 定位、自动/快捷键划词、截图结果承载、截图遮罩、置顶、转到主窗口和双窗口配置。不得创建侧悬浮结果窗口；截图遮罩是一次性捕获表面，不是第三个产品窗口。macOS 完整验收后移植 Windows，最后完成 Linux X11 与 Wayland 能力矩阵。

### Epic 7：OCR 管线

覆盖 `OCR-02`—`OCR-07`、`SET-09` 中的 OCR 配置。

交付本地 OCR、截图翻译、静默 OCR、十二种基线语言 fixture、指定语言重识别，以及无文本、权限拒绝和引擎失败的可执行错误状态。

### Epic 8：词典与语音

覆盖 `SRV-01`—`SRV-03`、`TTS-01`—`TTS-08`、`ACT-08`、`ACT-14`、`SET-09` 中的 TTS 配置。

交付 Apple Dictionary 的 macOS 明示能力、跨平台 MDict、平台系统 TTS、Bing/Google/有道/百度 TTS、自动/手动朗读和外部词典调用。Apple Translate 属于已确认的产品排除项。

### Epic 9：平台集成与完整硬化

重新验证 `APP-01`—`APP-08`、`CAP-01`—`CAP-10`、`ACT-01`—`ACT-14`、`SET-01`—`SET-13`、`NFR-01`—`NFR-08` 在各自平台范围的组合行为。

建立浏览器、Office、PDF、编辑器、聊天软件和系统应用兼容矩阵；覆盖权限恢复、剪贴板保护、焦点、多显示器、安装升级、深度链接、单实例、自动更新、诊断包和可访问性。

### Epic 10：公开发布

对 `OCR-01`—`OCR-07`、`QRY-01`—`QRY-10`、`SRV-01`—`SRV-23`、`TTS-01`—`TTS-08` 以及所有跨 Epic 行为执行最终回归。

完成 macOS 签名与 notarization、Windows code signing、Linux 包签名、签名 updater、数据库兼容回滚、SBOM、许可证检查和发布说明。只有功能对齐矩阵中的所有适用条目都有证据并通过，Phase 1 才能关闭。

## 完成判定

某一 Epic 只有在其 requirement ID 全部链接到通过证据、三个平台的共享构建通过、目标平台人工矩阵通过、已知限制被准确公开后才能关闭。Phase 1 关闭不自动启动 Phase 2；复习模式与 AI 学习系统需要新的已接受需求和架构决策。
