export const workspacePages = [
  { id: "overview", label: "启动与运行", title: "运行总览", description: "准备好声音，让今天的直播从这里开始。", group: "工作台", icon: "overview" },
  { id: "setup", label: "环境与模型", title: "环境与模型", description: "检查运行环境，准备你喜欢的语音模型。", group: "工作台", icon: "setup" },
  { id: "speech", label: "语音播报", title: "语音播报", description: "写下一句话，交给你的角色表达。", group: "内容与互动", icon: "speech" },
  { id: "resources", label: "角色与音色", title: "角色与音色", description: "选择角色、调好声音，赋予每次出场独特的个性。", group: "内容与互动", icon: "resources" },
  { id: "agent", label: "Agent 互动", title: "Agent 互动", description: "设定人设与话题，让角色自然回应观众。", group: "内容与互动", icon: "agent" },
  { id: "live", label: "直播连接", title: "直播连接", description: "连接直播间，接收弹幕与礼物。", group: "直播制作", icon: "live" },
  { id: "obs", label: "OBS 控制", title: "OBS 控制", description: "在这里切换场景，记录精彩时刻。", group: "直播制作", icon: "obs" },
  { id: "training", label: "训练与离线", title: "训练与离线", description: "微调专属音色，管理模型版本与离线运行。", group: "进阶工具", icon: "training" },
] as const;
export type WorkspacePage = typeof workspacePages[number]["id"];
export function pageFromHash(hash: string): WorkspacePage {
  if (hash === "#history-heading") return "speech";
  return workspacePages.find(page => `#${page.id}` === hash)?.id ?? "overview";
}
