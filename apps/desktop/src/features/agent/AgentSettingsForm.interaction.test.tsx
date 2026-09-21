import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { agentStatus } from "../../test/agent-fixtures";
import { AgentSettingsForm } from "./AgentSettingsForm";

it("保存读弹幕策略、欢迎开关及拥挤阈值，并将欢迎间隔转换成毫秒", async () => {
  const onSave = vi.fn().mockResolvedValue(true);
  render(<AgentSettingsForm settings={agentStatus().settings} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText("读弹幕策略"), { target: { value: "selective" } });
  fireEvent.click(screen.getByLabelText("欢迎进房观众"));
  for (const [label, value] of [
    ["每分钟弹幕阈值", "9"], ["每分钟进房阈值", "5"], ["待处理事件阈值", "7"],
    ["欢迎间隔（秒）", "45"], ["同一观众欢迎间隔（秒）", "900"],
  ]) fireEvent.change(screen.getByLabelText(label), { target: { value } });
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  await waitFor(() => expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ interaction: {
    chat_read_mode: "selective", welcome_enabled: false,
    busy_chat_count: 9, busy_enter_count: 5, busy_pending_count: 7,
    welcome_cooldown_ms: 45_000, welcome_viewer_cooldown_ms: 900_000,
  } })));
});

it("状态轮询保留互动草稿，保存失败后仍可修改", async () => {
  const onSave = vi.fn().mockResolvedValue(false);
  const settings = agentStatus().settings;
  const view = render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText("读弹幕策略"), { target: { value: "all" } });
  fireEvent.change(screen.getByLabelText("每分钟弹幕阈值"), { target: { value: "11" } });
  view.rerender(<AgentSettingsForm settings={{ ...settings, interaction: { ...settings.interaction, busy_chat_count: 20 } }} disabled={false} onSave={onSave} />);
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  await waitFor(() => expect(onSave).toHaveBeenCalledOnce());
  expect(screen.getByLabelText("读弹幕策略")).toHaveValue("all");
  expect(screen.getByLabelText("每分钟弹幕阈值")).toHaveValue(11);
});

it.each([
  ["每分钟弹幕阈值", "0"], ["每分钟弹幕阈值", "1001"],
  ["每分钟进房阈值", "1.5"], ["每分钟进房阈值", "1001"],
  ["待处理事件阈值", "513"], ["欢迎间隔（秒）", "0"],
  ["欢迎间隔（秒）", "3601"], ["同一观众欢迎间隔（秒）", "86401"],
])("%s 为 %s 时保留草稿并拒绝保存", async (label, value) => {
  const onSave = vi.fn().mockResolvedValue(true);
  render(<AgentSettingsForm settings={agentStatus().settings} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  expect(await screen.findByRole("alert")).toBeVisible();
  expect(onSave).not.toHaveBeenCalled();
  expect(screen.getByLabelText(label)).toHaveValue(Number(value));
});
