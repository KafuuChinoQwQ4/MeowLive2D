export const workspacePages = [
  { id: "overview", label: "启动与运行", title: "运行总览", description: "管理服务与连接。", group: "工作台", icon: "overview" },
  { id: "setup", label: "环境与模型", title: "环境与模型", description: "检查环境，选择语音模型。", group: "工作台", icon: "setup" },
  { id: "speech", label: "语音播报", title: "语音播报", description: "输入文字，播放声音。", group: "内容与互动", icon: "speech" },
  { id: "resources", label: "角色与音色", title: "角色与音色", description: "管理形象、声音与动作。", group: "内容与互动", icon: "resources" },
  { id: "agent", label: "Agent 互动", title: "Agent 互动", description: "设置人设，回应观众。", group: "内容与互动", icon: "agent" },
  { id: "viewers", label: "观众记录", title: "观众与事件", description: "查看已保存的观众昵称与直播事件。", group: "内容与互动", icon: "viewers" },
  { id: "llm", label: "LLM 接入", title: "LLM 接入", description: "连接云端或本地大模型。", group: "内容与互动", icon: "setup" },
  { id: "live", label: "直播连接", title: "直播连接", description: "连接直播间，接收弹幕与礼物。", group: "直播制作", icon: "live" },
  { id: "obs", label: "OBS 控制", title: "OBS 控制", description: "切换场景，管理录制。", group: "直播制作", icon: "obs" },
  { id: "training", label: "训练与离线", title: "训练与离线", description: "训练音色，管理模型与版本。", group: "进阶工具", icon: "training" },
  { id: "guide", label: "使用指南", title: "使用指南", description: "新手入门与功能用法。", group: "帮助", icon: "guide" },
] as const;
export type WorkspacePage = typeof workspacePages[number]["id"];
export function pageFromHash(hash: string): WorkspacePage {
  if (hash === "#history-heading") return "speech";
  return workspacePages.find(page => `#${page.id}` === hash)?.id ?? "overview";
}
