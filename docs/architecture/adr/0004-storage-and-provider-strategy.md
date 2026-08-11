# ADR-0004：存储与 Provider 策略

- 状态：Accepted
- 决策日期：2026-08-11
- 决策者：项目所有者

## 背景

阶段一需要可靠保存设置、窗口 profile、词典索引和可选查询历史，并接入大量翻译、词典、AI 与 TTS 服务。直接从 WebView 访问数据库或在需求稳定前开放动态插件都会扩大安全和兼容成本。

## 决策

SQLite 是阶段一结构化持久化方案，连接、迁移、事务与查询只由 Rust storage 层拥有。敏感凭据仅保存到平台凭据库，SQLite 只保存不可逆推出秘密的引用。

阶段一 provider 在编译期注册，并按 `DictionaryProvider`、`TranslationProvider`、`AiProvider`、`SpeechProvider` 等能力接口实现。UI 消费 descriptor 生成配置界面，Rust 执行最终校验。阶段一不加载第三方动态二进制或脚本插件。

## 结果

- 数据迁移、并发访问和隐私策略有单一所有者。
- Provider 可以替换和测试，而第三方协议不会渗透到领域与 UI。
- 新增内置 provider 需要随应用发布，但避免了动态代码执行与插件 ABI 风险。

## 被否决的方案

- 前端 SQLite 插件：破坏 Rust 核心的数据所有权与事务边界。
- 每个 provider 自行持久化：无法统一迁移、凭据和诊断策略。
- 阶段一动态插件市场：需求与安全模型尚不足以承诺稳定插件 ABI。

## 验证要求

- migration 在窗口显示前运行，失败进入可诊断的安全启动状态。
- repository 测试覆盖事务、迁移和隐私保留策略。
- provider 契约测试覆盖配置校验、取消、超时、异常响应与脱敏错误。
- 安全测试证明密钥不进入 SQLite、日志、IPC 或诊断包。
