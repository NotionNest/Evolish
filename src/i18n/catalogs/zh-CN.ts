import type { MessageCatalog } from "../messages";

export const zhCNMessages = {
  "startup.eyebrow": "桌面学习工作区",
  "startup.purpose": "在一个专注的工作区中完成翻译、上下文采集与学习。",
  "startup.loading": "正在启动应用核心…",
  "error.ipc.unavailable": "无法连接到应用核心。",
  "error.unexpected": "应用发生了意外错误。",
  "runtime.core": "核心",
  "runtime.connected": "已连接",
  "runtime.version": "版本",
  "runtime.platform": "平台",
  "errorBoundary.title": "Evolish 遇到了意外错误",
  "errorBoundary.description": "当前视图无法安全继续，请重新加载应用以恢复。",
  "errorBoundary.reload": "重新加载应用",
} as const satisfies MessageCatalog;
