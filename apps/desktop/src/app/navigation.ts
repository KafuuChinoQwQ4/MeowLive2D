export const workspacePages = [
  { id: "overview", label: "启动与运行", title: "运行总览", group: "工作台", icon: "overview" },
  { id: "setup", label: "环境与模型", title: "环境与模型", group: "工作台", icon: "setup" },
  { id: "resources", label: "角色与人物卡", title: "角色与人物卡", group: "角色设置", icon: "resources" },
  { id: "agent", label: "Agent 互动", title: "Agent 互动", group: "Agent 与智能", icon: "agent" },
  { id: "agent-observability", label: "Agent 观察", title: "Agent 调度观察", group: "Agent 与智能", icon: "trace" },
  { id: "llm", label: "LLM 接入", title: "LLM 接入", group: "Agent 与智能", icon: "setup" },
  { id: "speech", label: "语音播报", title: "语音播报", group: "声音与播报", icon: "speech" },
  { id: "training", label: "声音训练", title: "音色管理与声音训练", group: "声音与播报", icon: "training" },
  { id: "live", label: "直播连接", title: "直播连接", group: "直播制作", icon: "live" },
  { id: "viewers", label: "观众记录", title: "观众与事件", group: "直播制作", icon: "viewers" },
  { id: "obs", label: "OBS 控制", title: "OBS 控制", group: "直播制作", icon: "obs" },
  { id: "logs", label: "运行日志", title: "运行日志", group: "帮助", icon: "trace" },
  { id: "guide", label: "使用指南", title: "使用指南", group: "帮助", icon: "guide" },
] as const;
export type WorkspacePage = typeof workspacePages[number]["id"];
export function pageFromHash(hash: string): WorkspacePage {
  if (hash === "#history-heading") return "speech";
  return workspacePages.find(page => `#${page.id}` === hash)?.id ?? "overview";
}
