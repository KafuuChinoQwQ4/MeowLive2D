export const workspacePages = [
  { id: "overview", label: "启动与运行", title: "运行总览", description: "管理服务与连接。", group: "工作台", icon: "overview" },
  { id: "setup", label: "环境与模型", title: "环境与模型", description: "检查环境，选择语音模型。", group: "工作台", icon: "setup" },
  { id: "resources", label: "角色与人物卡", title: "角色与人物卡", description: "设置角色形象、动作与主播人物卡。", group: "角色设置", icon: "resources" },
  { id: "agent", label: "Agent 互动", title: "Agent 互动", description: "设置互动策略，回应观众。", group: "Agent 与智能", icon: "agent" },
  { id: "agent-observability", label: "Agent 观察", title: "Agent 调度观察", description: "查看调度、模型 Turn、工具调用与语音播放链路。", group: "Agent 与智能", icon: "trace" },
  { id: "llm", label: "LLM 接入", title: "LLM 接入", description: "连接云端或本地大模型。", group: "Agent 与智能", icon: "setup" },
  { id: "speech", label: "语音播报", title: "语音播报", description: "输入文字，播放声音。", group: "声音与播报", icon: "speech" },
  { id: "training", label: "声音训练", title: "音色管理与声音训练", description: "上传参考音色，训练、试听并管理声音版本。", group: "声音与播报", icon: "training" },
  { id: "live", label: "直播连接", title: "直播连接", description: "连接直播间，接收弹幕与礼物。", group: "直播制作", icon: "live" },
  { id: "viewers", label: "观众记录", title: "观众与事件", description: "查看已保存的观众昵称与直播事件。", group: "直播制作", icon: "viewers" },
  { id: "obs", label: "OBS 控制", title: "OBS 控制", description: "切换场景，管理录制。", group: "直播制作", icon: "obs" },
  { id: "guide", label: "使用指南", title: "使用指南", description: "新手入门与功能用法。", group: "帮助", icon: "guide" },
] as const;
export type WorkspacePage = typeof workspacePages[number]["id"];
export function pageFromHash(hash: string): WorkspacePage {
  if (hash === "#history-heading") return "speech";
  return workspacePages.find(page => `#${page.id}` === hash)?.id ?? "overview";
}
