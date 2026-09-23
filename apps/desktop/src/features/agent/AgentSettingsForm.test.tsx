import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { agentStatus } from "../../test/agent-fixtures";
import { AgentSettingsForm } from "./AgentSettingsForm";

const settings = agentStatus().settings;

it("互动设置保存话题与冷却时间，并保留现有人设", async () => {
  const onSave = vi.fn().mockResolvedValue(true);
  render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText("直播话题"), { target: { value: "动作游戏" } });
  fireEvent.change(screen.getByLabelText("冷却时间（秒）"), { target: { value: "45" } });
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  await waitFor(() => expect(onSave).toHaveBeenCalledWith({ ...settings, topic: "动作游戏", cooldown_ms: 45_000 }));
  expect(screen.getByRole("link", { name: "角色与人物卡" })).toHaveAttribute("href", "#resources");
});

it("话题长度与冷却时间错误时保留草稿", async () => {
  const onSave = vi.fn().mockResolvedValue(true);
  render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText("直播话题"), { target: { value: "猫".repeat(201) } });
  fireEvent.change(screen.getByLabelText("冷却时间（秒）"), { target: { value: "0.5" } });
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("话题最多 200");
  expect(screen.getByRole("alert")).toHaveTextContent("1 到 3600");
  expect(screen.getByLabelText("冷却时间（秒）")).toHaveValue(0.5);
  expect(onSave).not.toHaveBeenCalled();
});

it("读取最新人物卡失败时在表单显示错误并保留互动草稿", async () => {
  const onSave = vi.fn().mockRejectedValue(new Error("Agent 状态读取失败"));
  render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText("直播话题"), { target: { value: "草稿话题" } });
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Agent 状态读取失败");
  expect(screen.getByLabelText("直播话题")).toHaveValue("草稿话题");
});
