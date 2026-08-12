# ADR-0006：主窗口与迷你窗口双窗口模型

- 状态：Accepted
- 决策日期：2026-08-12
- 决策者：项目所有者
- 产品记录：[Product Decisions](../../reference/pm/product-decisions.md)

## 背景

早期架构沿用 Easydict 的主窗口、侧悬浮窗口和迷你窗口三类结果窗口，并把设置规划为独立窗口。产品所有者确认该模型存在职责重叠：快捷键划词、截图翻译和自动划词不需要不同的结果窗口，独立设置窗口也会扩大状态同步和权限面。

## 决策

Evolish 只有两个持久化内容窗口角色：

- `Main`：手动输入、完整结果、历史、收藏、Provider/模式管理、全部设置和后续学习界面。
- `Mini`：自动划词、快捷键划词和截图翻译的统一上下文窗口；默认紧凑，提供适用的结果操作，并可把当前查询会话转到 Main。

明确排除：

- 不创建 `Floating`/侧悬浮结果窗口。
- 不创建独立 `Settings` WebView；设置是 Main 内的路由或面板。
- 不为已排除窗口保留空配置、权限、label 或兼容壳。

区域截图可使用一次性 `Capture Overlay`。它只负责选择区域，完成或取消后销毁，不显示翻译结果，不计为第三个产品窗口。

## 行为映射

| 入口 | 结果窗口 |
|---|---|
| 手动输入、输入翻译快捷键 | Main |
| 自动划词 | Mini |
| 快捷键划词 | Mini |
| 截图翻译 | Mini |
| 深度链接 | 默认 Main；调用参数未来可明确请求 Mini |
| 历史、收藏、设置、服务管理 | Main |

## 架构结果

- `WindowCoordinator` 只管理 `Main`、`Mini` 和瞬时 Capture Overlay 生命周期。
- 只有 main/mini 两个稳定 window profile；持久化层以封闭约束拒绝其他角色，两者可拥有不同查询配置。
- Main 与 Mini 使用独立最小 Tauri capability；Mini 永远没有凭据写入、Provider 管理、历史清理或设置修改权限。
- Rust 核心持有查询 session；“转到主窗口”转移/附加同一 session，不重新请求 AI。
- 自动划词、快捷键划词和截图翻译共用 Mini 定位、焦点、置顶与多显示器规则。

## 被否决的方案

- 保留侧悬浮窗口但默认关闭：仍需维护窗口、状态、权限、测试和设置。
- 快捷键划词打开 Main：会打断当前阅读/写作上下文，不符合低打扰原则。
- 为设置保留独立窗口：增加 WebView 与权限面，且没有独立生命周期价值。
- 把 Capture Overlay 当作普通窗口：可能错误获得查询或设置权限，并使窗口模型含混。

## 验证要求

- 代码、Tauri 配置、capability、window label 和持久化 profile 中不存在 Floating/侧悬浮或独立 Settings 窗口。
- 三类上下文入口（自动划词、快捷键划词、截图翻译）都复用 Mini，并通过连续查询和多显示器测试。
- Mini 转到 Main 不产生新的 Provider attempt。
- Main 与 Mini 权限隔离测试通过；Capture Overlay 只能调用截图会话命令。
