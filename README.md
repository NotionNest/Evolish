# Evolish

Evolish 是一个面向 macOS、Windows 与 Linux 的翻译和学习桌面应用。当前阶段首先完整实现 Easydict 的功能范围，后续在同一数据与应用核心上加入 AI 和复习学习系统。

## 技术栈

- Tauri 2 + Rust：桌面容器、业务核心与系统能力
- React 19 + TypeScript + Vite：多窗口界面
- pnpm：前端依赖与任务管理
- Vitest + React Testing Library：前端测试
- Rust unit tests + Clippy：核心测试与静态检查

详细决策见：

- [技术选型](docs/architecture/technology-selection.md)
- [总体架构](docs/architecture/overall-architecture.md)
- [阶段一 PRD](docs/reference/pm/phase-1-prd.md)

## 环境要求

- Node.js 22.x（仓库通过 `.node-version` 固定主版本）
- pnpm 11
- Rust 1.97.1（仓库通过 `rust-toolchain.toml` 固定）
- 对应平台的 Tauri 2 系统依赖

## 本地开发

```bash
pnpm install
pnpm tauri dev
```

## 质量检查

```bash
pnpm check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

分支、提交和 Pull Request 规范见 [CONTRIBUTING.md](CONTRIBUTING.md)。提交消息与 PR 标题使用 Conventional Commits，并由本地 Git hook 和 CI 同时校验。

## 代码边界

```text
src/
├── app/          React 应用装配与窗口根组件
├── bridge/       唯一的 Tauri IPC 前端边界
├── styles/       全局设计令牌与基础样式
└── test/         前端测试环境

src-tauri/src/
├── domain/       不依赖 Tauri 的领域类型与规则
├── application/  应用用例与协调逻辑
└── ipc/          最小化 Tauri command 边界
```

业务增长时应按总体架构增加 `features`、`providers`、`platform`、`storage`、`security` 与 `windows` 模块，不允许 React 直接访问数据库、系统凭据或外部服务密钥。
