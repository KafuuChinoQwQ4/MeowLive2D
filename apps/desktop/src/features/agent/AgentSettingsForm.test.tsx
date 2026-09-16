import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AgentSettingsForm } from "./AgentSettingsForm";

const settings = { persona: "温柔的猫娘主播", topic: "轻松聊天", proactive_enabled: false, cooldown_ms: 30_000 };

describe("Agent 设置表单", () => {
  it("轮询状态更新时保留用户尚未保存的草稿", () => {
    const view = render(<AgentSettingsForm settings={settings} disabled={false} onSave={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("主播人设"), { target: { value: "正在编辑的人设" } });

    view.rerender(<AgentSettingsForm settings={{ ...settings, persona: "服务端旧值" }} disabled={false} onSave={vi.fn()} />);

    expect(screen.getByLabelText("主播人设")).toHaveValue("正在编辑的人设");
  });

  it("校验人设、话题与 1 到 3600 秒的整数冷却时间并保留输入", async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
    fireEvent.change(screen.getByLabelText("主播人设"), { target: { value: "  " } });
    fireEvent.change(screen.getByLabelText("冷却时间（秒）"), { target: { value: "0.5" } });
    fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("人设");
    expect(screen.getByRole("alert")).toHaveTextContent("1 到 3600");
    expect(screen.getByLabelText("主播人设")).toHaveValue("  ");
    expect(onSave).not.toHaveBeenCalled();
  });

  it("按毫秒契约提交设置并明确保存会暂停", async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<AgentSettingsForm settings={settings} disabled={false} onSave={onSave} />);
    fireEvent.change(screen.getByLabelText("主播人设"), { target: { value: "冷静主持人" } });
    fireEvent.change(screen.getByLabelText("直播话题"), { target: { value: "动作游戏" } });
    fireEvent.change(screen.getByLabelText("冷却时间（秒）"), { target: { value: "45" } });
    fireEvent.click(screen.getByLabelText("允许空闲时主动发言"));
    fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));

    await waitFor(() => expect(onSave).toHaveBeenCalledWith({
      persona: "冷静主持人", topic: "动作游戏", proactive_enabled: true, cooldown_ms: 45_000,
    }));
    expect(screen.getByText(/保存设置会暂停 Agent/)).toBeVisible();
  });
});
