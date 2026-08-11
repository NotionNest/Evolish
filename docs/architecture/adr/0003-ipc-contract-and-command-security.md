# ADR-0003：IPC 契约与命令安全

- 状态：Accepted
- 决策日期：2026-08-11
- 决策者：项目所有者

## 背景

React WebView 与 Rust 核心之间的手写重复类型容易漂移；Tauri 命令若默认暴露给所有窗口，会让轻量窗口获得不需要的设置或敏感能力。领域模型也不应因序列化需求直接成为公开协议。

## 决策

Rust IPC DTO 是跨边界契约的唯一来源。DTO 与领域/应用对象分离，使用 Serde 定义序列化，并通过 `ts-rs` 生成 TypeScript 类型。生成文件纳入版本控制，CI 检查漂移。

每个 Tauri 命令使用明确名称并登记到应用 manifest；capability 只向需要的窗口授予生成的 allow permission。禁止通用 shell、任意文件、任意 SQL 和通用 execute 命令。错误通过稳定的 `AppErrorDto` 传递，底层 cause 不进入 WebView。

## 结果

- 修改 IPC 字段时 Rust 测试和 TypeScript 编译共同暴露不兼容。
- 每类窗口的攻击面由可审查的 capability 文件决定。
- 领域对象可以独立演进，不泄露存储或 provider 内部字段。

## 被否决的方案

- Rust/TypeScript 双份手写类型：无法可靠防止漂移。
- 直接序列化领域对象：边界与内部模型耦合。
- 将全部命令授予所有窗口：违反最小权限原则。

## 验证要求

- Rust 映射测试验证应用对象到 DTO 的显式转换。
- 导出测试生成 TypeScript，CI 对生成目录执行无差异检查。
- capability 测试验证命令只授予声明的窗口且没有 deny/allow 冲突。
