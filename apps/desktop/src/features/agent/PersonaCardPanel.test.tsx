import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import type { AgentClient } from "../../services/server/agent";
import { agentStatus } from "../../test/agent-fixtures";
import { PersonaCardPanel } from "./PersonaCardPanel";

function setup(persona = "温柔的猫娘主播") {
  let current = agentStatus({ settings: { ...agentStatus().settings, persona } });
  let profiles = { profiles: [{ id: "card-1", name: "配置1", persona }], selected_profile_id: "card-1", storage_available: true };
  const client = {
    baseUrl: "http://127.0.0.1:19600",
    getStatus: vi.fn(async () => current),
    saveSettings: vi.fn(async settings => {
      current = agentStatus({ settings });
      profiles = { ...profiles, profiles: profiles.profiles.map(profile => profile.id === profiles.selected_profile_id ? { ...profile, persona: settings.persona } : profile) };
      return current;
    }),
    getPersonaProfiles: vi.fn(async () => profiles),
    createPersonaProfile: vi.fn(async ({ name, persona }) => {
      const id = `card-${profiles.profiles.length + 1}`;
      profiles = { ...profiles, profiles: [...profiles.profiles, { id, name, persona }], selected_profile_id: id };
      current = agentStatus({ settings: { ...current.settings, persona } });
      return profiles;
    }),
    selectPersonaProfile: vi.fn(async ({ id }) => {
      const persona = profiles.profiles.find(profile => profile.id === id)!.persona;
      profiles = { ...profiles, selected_profile_id: id };
      current = agentStatus({ settings: { ...current.settings, persona } });
      return profiles;
    }),
    renamePersonaProfile: vi.fn(async ({ id, name }) => {
      profiles = { ...profiles, profiles: profiles.profiles.map(profile => profile.id === id ? { ...profile, name } : profile) };
      return profiles;
    }),
    updatePersonaProfile: vi.fn(async ({ id, name, persona }) => {
      profiles = { ...profiles, profiles: profiles.profiles.map(profile => profile.id === id ? { ...profile, name, persona } : profile) };
      current = agentStatus({ settings: { ...current.settings, persona } });
      return profiles;
    }),
    deletePersonaProfile: vi.fn(async ({ id }) => {
      profiles = { ...profiles, profiles: profiles.profiles.filter(profile => profile.id !== id), selected_profile_id: "card-1" };
      current = agentStatus({ settings: { ...current.settings, persona: profiles.profiles[0].persona } });
      return profiles;
    }),
  } as unknown as AgentClient;
  const view = render(<PersonaCardPanel client={client} />);
  return { client, view, changeServerTopic: (topic: string) => { current = agentStatus({ settings: { ...current.settings, topic } }); } };
}

it("人物卡细调字段合成为 Agent 人设，并保留最新互动话题", async () => {
  const { client, changeServerTopic } = setup();
  await userEvent.click(await screen.findByText("展开人物卡"));
  fireEvent.change(screen.getByLabelText("核心身份"), { target: { value: "魔法少女主播" } });
  fireEvent.change(screen.getByLabelText("背景经历"), { target: { value: "来自月见镇的见习魔女" } });
  changeServerTopic("服务器上的新话题");
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(client.updatePersonaProfile).toHaveBeenCalledWith(expect.objectContaining({
    persona: "【人物卡 v1】\n【核心身份】\n魔法少女主播\n【背景经历】\n来自月见镇的见习魔女",
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
  await waitFor(() => expect(client.updatePersonaProfile).toHaveBeenCalledWith(expect.objectContaining({ persona: "正在编辑的人设" }), expect.anything()));
});

it("服务端未确认人物卡保存时保留草稿并显示错误", async () => {
  const { client } = setup();
  client.updatePersonaProfile = vi.fn(async () => ({ profiles: [{ id: "card-1", name: "配置1", persona: "仍在编辑的人设" }], selected_profile_id: "card-1", storage_available: true }));
  fireEvent.change(await screen.findByLabelText("核心身份"), { target: { value: "仍在编辑的人设" } });
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("未确认");
  expect(screen.getByLabelText("核心身份")).toHaveValue("仍在编辑的人设");
});

it("历史模型错误不影响已确认的人物卡保存", async () => {
  const { client } = setup();
  const status = client.getStatus;
  client.getStatus = vi.fn(async signal => ({ ...await status(signal), last_error: "此前的模型决策错误" }));
  fireEvent.change(await screen.findByLabelText("核心身份"), { target: { value: "新的持久人设" } });
  await userEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(screen.getByLabelText("核心身份")).toHaveValue("新的持久人设"));
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("保存人物卡时一并持久化已编辑的标题", async () => {
  const { client, view } = setup("原有人设");
  await screen.findByLabelText("核心身份");
  fireEvent.change(screen.getByLabelText("人物卡标题"), { target: { value: "准备改名" } });
  fireEvent.change(screen.getByLabelText("核心身份"), { target: { value: "新版人设" } });
  await userEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(screen.getByLabelText("核心身份")).toHaveValue("新版人设"));
  await waitFor(() => expect(client.updatePersonaProfile).toHaveBeenCalledWith({ id: "card-1", name: "准备改名", persona: "新版人设" }, expect.any(AbortSignal)));
  view.unmount();
  render(<PersonaCardPanel client={client} />);
  await screen.findByLabelText("核心身份");
  expect(screen.getByLabelText("人物卡标题")).toHaveValue("准备改名");
  expect(screen.getByRole("button", { name: "重命名人物卡" })).toBeDisabled();
});

it("切换人物卡前保护尚未保存的标题", async () => {
  const { client } = setup("原有人设");
  await screen.findByLabelText("核心身份");
  fireEvent.change(screen.getByLabelText("人物卡标题"), { target: { value: "草稿标题" } });
  vi.spyOn(window, "confirm").mockReturnValueOnce(false);
  await userEvent.click(screen.getByRole("button", { name: "已保存人物卡" }));
  await userEvent.click(screen.getByRole("button", { name: "新建人物卡" }));
  expect(screen.getByLabelText("人物卡标题")).toHaveValue("草稿标题");
  expect(client.createPersonaProfile).not.toHaveBeenCalled();
});

it("新建并切换人物卡后重进页面仍回填所选内容", async () => {
  const { client, view } = setup("原有人设");
  await screen.findByLabelText("核心身份");
  await userEvent.click(screen.getByRole("button", { name: "已保存人物卡" }));
  await userEvent.click(screen.getByRole("button", { name: "新建人物卡" }));
  fireEvent.change(screen.getByLabelText("人物卡标题"), { target: { value: "魔法少女" } });
  fireEvent.change(screen.getByLabelText("核心身份"), { target: { value: "月见镇魔女" } });
  await userEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(client.createPersonaProfile).toHaveBeenCalledWith({ name: "魔法少女", persona: "月见镇魔女" }, expect.any(AbortSignal)));
  view.unmount();
  render(<PersonaCardPanel client={client} />);
  expect(await screen.findByLabelText("核心身份")).toHaveValue("月见镇魔女");
  expect(screen.getByLabelText("人物卡标题")).toHaveValue("魔法少女");
  await userEvent.click(screen.getByRole("button", { name: "已保存人物卡" }));
  await userEvent.click(screen.getByRole("button", { name: /配置1.*原有人设/ }));
  expect(await screen.findByLabelText("核心身份")).toHaveValue("原有人设");
  expect(client.selectPersonaProfile).toHaveBeenCalledWith({ id: "card-1" }, expect.any(AbortSignal));
});

it("已保存人物卡可重命名和删除，删除当前卡后显示回退卡", async () => {
  const { client } = setup("原有人设");
  await screen.findByLabelText("核心身份");
  fireEvent.change(screen.getByLabelText("人物卡标题"), { target: { value: "老朋友" } });
  await userEvent.click(screen.getByRole("button", { name: "重命名人物卡" }));
  await waitFor(() => expect(client.renamePersonaProfile).toHaveBeenCalledWith({ id: "card-1", name: "老朋友" }, expect.any(AbortSignal)));
  await userEvent.click(screen.getByRole("button", { name: "已保存人物卡" }));
  await userEvent.click(screen.getByRole("button", { name: "新建人物卡" }));
  fireEvent.change(screen.getByLabelText("人物卡标题"), { target: { value: "新角色" } });
  fireEvent.change(screen.getByLabelText("核心身份"), { target: { value: "新角色人设" } });
  await userEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(screen.getByLabelText("人物卡标题")).toHaveValue("新角色"));
  vi.spyOn(window, "confirm").mockReturnValueOnce(true);
  await userEvent.click(screen.getByRole("button", { name: "已保存人物卡" }));
  await userEvent.click(screen.getByRole("button", { name: "删除人物卡" }));
  expect(await screen.findByLabelText("核心身份")).toHaveValue("原有人设");
  expect(screen.getByLabelText("人物卡标题")).toHaveValue("老朋友");
  expect(client.deletePersonaProfile).toHaveBeenCalledWith({ id: "card-2" }, expect.any(AbortSignal));
});
