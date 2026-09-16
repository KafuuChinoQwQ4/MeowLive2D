import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { AgentSettingsForm } from "./AgentSettingsForm";

const settings = { persona: "猫咪主播", topic: "", proactive_enabled: false, cooldown_ms: 30000 };

it("默认空话题允许保存，与后端约束一致", async () => {
  const onSave = vi.fn().mockResolvedValue(true);
  render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  await waitFor(() => expect(onSave).toHaveBeenCalledWith(settings));
});

it.each([["主播人设", "猫".repeat(2001)], ["直播话题", "猫".repeat(201)]])("%s 按 Unicode 字符限制长度", async (label, value) => {
  const onSave = vi.fn().mockResolvedValue(true);
  render(<AgentSettingsForm settings={{ ...settings, topic: "游戏" }} disabled={false} onSave={onSave} />);
  fireEvent.change(screen.getByLabelText(label), { target: { value } });
  fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
  expect(await screen.findByRole("alert")).toBeVisible();
  expect(onSave).not.toHaveBeenCalled();
});
