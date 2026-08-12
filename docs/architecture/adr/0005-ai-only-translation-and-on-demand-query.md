# ADR-0005：AI-only 翻译与按需 Provider 请求

- 状态：Accepted
- 决策日期：2026-08-11
- 决策者：项目所有者
- 产品依据：[文本输入翻译 PRD](../../reference/pm/text-input-translation-prd.md)

## 背景

原始 Phase 1 基线按 Easydict 的服务目录规划传统翻译、AI、词典和系统翻译 provider，并以多个服务默认并行作为查询编排目标。产品所有者确认 Evolish 的翻译、释义、例句和语言分析只使用 AI；首次请求只调用主 AI 服务，其他服务仅在用户明确展开时请求，且正文完整生成后才展示。

这项决策改变 provider 范围、查询监督器的默认调度、结果事件模型和历史结构，需要在实现前固化架构边界。

## 决策

### Provider 范围

- 原生协议 adapter：OpenAI、Anthropic、Gemini。
- 通用 adapter：OpenAI-compatible 自定义服务，允许创建多个命名 profile、配置自定义地址、模型、参数和凭据引用。
- DeepSeek、Ollama、Groq、智谱 AI、GitHub Models 等兼容服务通过自定义 profile 使用，不维护品牌专属协议 adapter。
- 不实现 Apple Translate、DeepL、Google Translate、有道翻译、腾讯翻译、Bing 翻译、百度翻译、小牛、彩云、阿里翻译、火山翻译和豆包传统翻译 adapter。
- MDict、系统词典和 TTS 是独立能力，不进入 AI 翻译 provider 的结果生成链路。

### 查询编排

- 每个窗口 profile 指定一个已启用的主 AI provider profile。
- 用户提交后，`QuerySupervisor` 只创建主服务 attempt。
- 其他已启用服务以折叠的 `NotRequested` 状态出现；用户首次展开时才创建对应 attempt。
- 主服务失败、超时、空结果或格式错误时不自动回退。
- 重试只作用于用户明确选择的服务，并创建新的 attempt ID。
- 新查询取消旧查询，并通过 session ID、attempt ID 和窗口 generation 隔离无法及时取消的迟到响应。

### 响应与展示

- provider adapter 将完整响应解析为版本化 `AdaptiveTranslationResult`。
- WebView 只接收状态事件和通过校验的完整结果，不接收用于渲染的正文 token 流。
- 生成期间状态为 `Running`；完整响应通过结构和完整性校验后，以单次完成事件进入 `Succeeded`。
- 非法、空白或截断结构进入明确错误状态，不展示残缺正文。
- 超长文本按自然段形成有序子任务；全部子任务成功并合并校验后才发布完整结果。

### 历史与收藏

- 成功结果默认写入本地历史，隐私设置可关闭写入或启用临时隐私模式。
- 每次重试形成同一 session 下的新 attempt 和结果版本，旧成功结果保留。
- 收藏引用稳定的 session、attempt 和结果版本。
- API 密钥只保存在平台凭据库，SQLite 仅保存 `secret_ref`。

## 结果

- 默认请求数量从“所有启用服务”降为“主服务一个”，费用和行为可预测。
- UI 与查询核心必须表达 `NotRequested`，不能把未请求误报为空结果或失败。
- Provider Registry 从品牌目录转向少量原生协议加通用兼容协议。
- 完整结果事件更适合持久化、收藏和未来知识卡引用，但首个正文出现时间受模型完整生成时间影响。
- 传统翻译服务不再是 Easydict 功能对齐的适用项，必须用“产品排除”状态保留审计记录。

## 被否决的方案

- 默认并行请求所有服务：费用和噪声不可控。
- 主服务失败后自动回退：会产生未经用户明确触发的外部请求。
- 只支持 OpenAI-compatible：无法对 Anthropic 与 Gemini 的原生能力、错误和结构化输出提供可靠契约。
- 向 UI 转发正文 token 流：与完整生成后一次展示的产品决策冲突。
- 将收藏直接建模为知识卡：提前耦合尚未设计的 Phase 2 学习领域。

## 验证要求

- 提交一次查询时，除主服务外的 provider 请求计数为零。
- 展开一个其他服务只创建该服务的一个 attempt。
- 主服务任何失败状态都不触发隐式回退。
- 迟到响应不能污染当前 session 或被写成当前成功历史。
- UI 在完整结果事件前不收到可渲染正文。
- provider 契约测试覆盖完整结构、非法结构、截断、超时、取消、限额、认证和安全拒绝。
- storage 测试覆盖 attempt 版本、历史关闭、隐私模式和收藏引用一致性。
