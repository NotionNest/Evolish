# Contributing to Evolish

## 开发准备

```bash
pnpm install
pnpm setup:git
```

`pnpm install` 会自动配置仓库内的 Git hooks 和提交消息模板。提交前运行：

```bash
pnpm check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## 分支规范

稳定分支为 `main`，功能开发通过短生命周期分支和 Pull Request 完成：

- `feature/<description>`：新功能
- `fix/<description>`：缺陷修复
- `refactor/<description>`：不改变外部行为的重构
- `docs/<description>`：文档变更
- `chore/<description>`：工程、依赖和维护任务

分支名使用小写英文和连字符，例如 `feature/selection-capture`。

## 提交规范

提交遵循 Conventional Commits：

```text
<type>(<optional-scope>): <description>
```

允许的 type：

- `feat`：新增用户能力
- `fix`：修复缺陷
- `docs`：仅文档变化
- `style`：不影响逻辑的格式变化
- `refactor`：既非新功能也非修复的代码调整
- `perf`：性能改进
- `test`：测试变化
- `build`：构建系统或外部依赖
- `ci`：持续集成配置
- `chore`：其他维护工作
- `revert`：回滚提交

示例：

```text
feat(query): add selection capture pipeline
fix(window): preserve focus when showing mini window
docs(architecture): record SQLite migration strategy
```

不兼容变更在 type/scope 后添加 `!`，并在 footer 中说明 `BREAKING CHANGE:`。标题最多 100 个字符。

## Pull Request 规范

- PR 标题同样使用 Conventional Commits。
- 一个 PR 只解决一个明确问题。
- PR 描述必须说明变更内容、原因、验证方式和平台差异。
- 合并前必须通过前端、Rust 和提交规范检查。
- 默认使用 squash merge，让 `main` 保持线性且每个 PR 对应一个语义提交。
