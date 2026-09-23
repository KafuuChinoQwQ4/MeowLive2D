import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import type { AgentClient } from "../../services/server/agent";
import { agentStatus } from "../../test/agent-fixtures";
import { PersonaCardPanel } from "./PersonaCardPanel";

function setup(persona = "温柔的猫娘主播") {
  let current = agentStatus({ settings: { ...agentStatus().settings, persona } });
  const client = {
    baseUrl: "http://127.0.0.1:19600",
    getStatus: vi.fn(async () => current),
    saveSettings: vi.fn(async settings => { current = agentStatus({ settings }); return current; }),
  } as unknown as AgentClient;
  render(<PersonaCardPanel client={client} />);
  return { client, changeServerTopic: (topic: string) => { current = agentStatus({ settings: { ...current.settings, topic } }); } };
}

it("人物卡细调字段合成为 Agent 人设，并保留最新互动话题", async () => {
  const { client, changeServerTopic } = setup();
  await userEvent.click(await screen.findByText("展开人物卡"));
  fireEvent.change(screen.getByLabelText("核心身份"), { target: { value: "魔法少女主播" } });
  fireEvent.change(screen.getByLabelText("背景经历"), { target: { value: "来自月见镇的见习魔女" } });
  changeServerTopic("服务器上的新话题");
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(client.saveSettings).toHaveBeenCalledWith(expect.objectContaining({
    persona: "【人物卡 v1】\n【核心身份】\n魔法少女主播\n【背景经历】\n来自月见镇的见习魔女",
    topic: "服务器上的新话题",
  }), expect.any(AbortSignal)));
});

it("重新打开结构化人物卡时还原字段", async () => {
  setup("【人物卡 v1】\n【核心身份】\n治愈系猫娘主播\n【与观众的关系】\n把观众当作一起冒险的朋友");
  expect(await screen.findByLabelText("核心身份")).toHaveValue("治愈系猫娘主播");
  await userEvent.click(screen.getByText("展开人物卡"));
  expect(screen.getByLabelText("与观众的关系")).toHaveValue("把观众当作一起冒险的朋友");
});

it("保存前的服务端读取不会覆盖未保存的人物卡草稿", async () => {
  const { client, changeServerTopic } = setup();
  fireEvent.change(await screen.findByLabelText("核心身份"), { target: { value: "正在编辑的人设" } });
  changeServerTopic("新话题");
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(client.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ persona: "正在编辑的人设", topic: "新话题" }), expect.anything()));
});

it("服务端未确认人物卡保存时保留草稿并显示错误", async () => {
  const { client } = setup();
  client.saveSettings = vi.fn(async () => agentStatus({ paused: false, phase: "waiting" }));
  fireEvent.change(await screen.findByLabelText("核心身份"), { target: { value: "仍在编辑的人设" } });
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("未确认");
  expect(screen.getByLabelText("核心身份")).toHaveValue("仍在编辑的人设");
});
