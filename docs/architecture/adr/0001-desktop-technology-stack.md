# ADR-0001：桌面端基础技术栈

- 状态：Accepted
- 决策日期：2026-08-11
- 决策者：项目所有者

## 背景

Evolish 阶段一需要在 macOS、Windows 和 Linux 上实现跨应用取词、全局快捷键、系统托盘、三类窗口、截图 OCR、TTS、本地词典和多个翻译服务并行查询。阶段二将在同一产品中增加 AI 知识卡和复习学习系统。因此，基础技术栈既要能深入访问操作系统，也要支持长期演进的内容型复杂界面。

## 决策

采用：

- Tauri 2 作为桌面应用容器与窗口/IPC 基础；
- Rust 作为系统集成、应用核心、查询编排、服务适配和持久化语言；
- React + TypeScript + Vite 作为用户界面技术栈。

Rust 核心是业务状态和敏感数据的唯一可信来源。React WebView 不直接访问数据库、系统凭据库或第三方服务密钥。

## 结果

### 正向结果

- 系统能力和跨平台业务核心可以使用同一种内存安全语言实现。
- UI 可以复用成熟的 Web 组件生态，并适应未来知识卡和学习系统。
- Tauri 使用操作系统 WebView，安装体积和常驻资源通常低于捆绑 Chromium 的方案。
- Tauri capability 可以按窗口限制权限。

### 代价

- 需要维护 Rust 与 TypeScript 边界，并自动生成共享 DTO 类型。
- 不同平台 WebView 存在渲染差异，必须执行三端 UI 验收。
- 深层系统能力仍需 macOS、Windows、Linux 的原生适配代码。
- Linux Wayland 的全局坐标和窗口定位限制无法由框架消除。

## 被否决的主要方案

- Qt 6 + C++/QML：系统桌面能力强，但单人长期维护和未来学习 UI 成本更高。
- Flutter Desktop：UI 一致性强，但大量平台能力仍需三端插件，业务核心语言边界更分散。
- Electron：生态成熟，但常驻资源和原生模块维护成本不符合本项目目标。

## 依据

- [Tauri Architecture](https://v2.tauri.app/concept/architecture/)
- [Tauri Process Model](https://v2.tauri.app/concept/process-model/)
- [Tauri Capabilities](https://v2.tauri.app/security/capabilities/)

