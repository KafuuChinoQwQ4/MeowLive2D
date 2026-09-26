import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { AgentClient } from "../../services/server/agent";
import { agentStatus } from "../../test/agent-fixtures";
import { PersonaCardPanel } from "./PersonaCardPanel";

function setup(persona = "猫咪主播") {
  let snapshot = agentStatus({ settings: { ...agentStatus().settings, persona } });
  const client = {
    baseUrl: "http://127.0.0.1:19600",
    getStatus: vi.fn(async () => snapshot),
    saveSettings: vi.fn(async settings => { snapshot = agentStatus({ settings }); return snapshot; }),
    getPersonaProfiles: vi.fn(async () => ({ profiles: [{ id: "card-1", name: "配置1", persona: snapshot.settings.persona }], selected_profile_id: "card-1", storage_available: true })),
    updatePersonaProfile: vi.fn(async ({ persona }) => { snapshot = agentStatus({ settings: { ...snapshot.settings, persona } }); return { profiles: [{ id: "card-1", name: "配置1", persona }], selected_profile_id: "card-1", storage_available: true }; }),
  } as unknown as AgentClient;
  render(<PersonaCardPanel client={client} />);
  return client;
}

it("核心身份按 Unicode 字符限制完整人物卡长度", async () => {
  const client = setup();
  fireEvent.change(await screen.findByLabelText("核心身份"), { target: { value: "猫".repeat(2001) } });
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("2000");
  expect(client.saveSettings).not.toHaveBeenCalled();
});

it("附加字段计入完整提示词的 2000 字限制", async () => {
  const client = setup();
  await screen.findByLabelText("核心身份");
  fireEvent.change(screen.getByLabelText("示例表达"), { target: { value: "猫".repeat(2000) } });
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("2000");
  expect(client.saveSettings).not.toHaveBeenCalled();
});

it("人物卡只填写附加项时仍要求核心身份", async () => {
  const client = setup();
  fireEvent.change(await screen.findByLabelText("核心身份"), { target: { value: "" } });
  fireEvent.change(screen.getByLabelText("性格特点"), { target: { value: "温柔" } });
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("核心身份");
  expect(client.saveSettings).not.toHaveBeenCalled();
});

it("表情字符计数按 Unicode 字符，不误报 2000 字内容", async () => {
  const client = setup("🐈".repeat(2000));
  expect(await screen.findByLabelText("核心身份")).not.toHaveAttribute("aria-invalid", "true");
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(client.updatePersonaProfile).toHaveBeenCalledWith(expect.objectContaining({ persona: "🐈".repeat(2000) }), expect.anything()));
});

it("以人物卡标题开头的旧自由文本仍原样保存", async () => {
  const persona = `【人物卡 v1】\n${"猫".repeat(1991)}`;
  const client = setup(persona);
  await screen.findByLabelText("核心身份");
  fireEvent.click(screen.getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(client.updatePersonaProfile).toHaveBeenCalledWith(expect.objectContaining({ persona }), expect.anything()));
});
