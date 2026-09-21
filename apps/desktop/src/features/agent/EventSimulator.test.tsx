import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EventSimulator } from "./EventSimulator";

describe("直播事件模拟与回放", () => {
  it("模拟 SC 保留人民币元金额、内容和毫秒有效期", async () => {
    const onSubmit = vi.fn().mockResolvedValue({ accepted: 1, duplicates: 0 });
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: "SC" }));
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: " 小猫 " } });
    fireEvent.change(screen.getByLabelText("SC 内容"), { target: { value: " 继续加油 " } });
    fireEvent.change(screen.getByLabelText("SC 金额（元）"), { target: { value: "50" } });
    fireEvent.change(screen.getByLabelText("SC 有效时长（秒）"), { target: { value: "120" } });
    const before = Date.now();
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));
    await screen.findByRole("status");
    const event = onSubmit.mock.calls[0][0].events[0];
    expect(event).toMatchObject({ source: "simulator", viewer: "小猫", kind: { type: "super_chat", text: "继续加油", amount_cny: 50 } });
    expect(event.kind.start_at_ms).toBeGreaterThanOrEqual(before);
    expect(event.kind.start_at_ms).toBeLessThanOrEqual(Date.now());
    expect(event.kind.end_at_ms - event.kind.start_at_ms).toBe(120_000);
    expect(screen.getByLabelText("SC 内容")).toHaveValue("");
  });

  it("模拟进房仅提交昵称和进房事件", async () => {
    const onSubmit = vi.fn().mockResolvedValue({ accepted: 1, duplicates: 0 });
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: "进房" }));
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));
    await screen.findByRole("status");
    expect(onSubmit.mock.calls[0][0].events[0]).toMatchObject({ source: "simulator", viewer: "小猫", kind: { type: "room_enter" } });
  });

  it("SC 提交失败时保留内容及金额", async () => {
    const onSubmit = vi.fn().mockResolvedValue(null);
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: "SC" }));
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.change(screen.getByLabelText("SC 内容"), { target: { value: "保留留言" } });
    fireEvent.change(screen.getByLabelText("SC 金额（元）"), { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledOnce());
    expect(screen.getByLabelText("SC 内容")).toHaveValue("保留留言");
    expect(screen.getByLabelText("SC 金额（元）")).toHaveValue(30);
  });

  it("回放 SC 与进房保留原始载荷", async () => {
    const onSubmit = vi.fn().mockResolvedValue({ accepted: 2, duplicates: 0 });
    const events = [
      { id: "sc-1", source: "recording", viewer: "小猫", kind: { type: "super_chat", text: "加油", amount_cny: 30, start_at_ms: 1_700_000_000_000, end_at_ms: 1_700_000_060_000 } },
      { id: "enter-1", source: "recording", viewer: "小鱼", kind: { type: "room_enter" } },
    ];
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("事件回放 JSON"), { target: { value: JSON.stringify(events) } });
    fireEvent.click(screen.getByRole("button", { name: "提交事件回放" }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith({ events }));
    expect(await screen.findByRole("status")).toHaveTextContent("已接收 2 条事件");
  });

  it.each([{ type: "room_enter" }, { type: "super_chat", text: "加油", amount_cny: 30, start_at_ms: 1000, end_at_ms: 2000 }])("拒绝 $type 回放混入礼物元数据", async (kind) => {
    const onSubmit = vi.fn();
    const value = JSON.stringify([{ id: "event", source: "recording", viewer: "小猫", kind, gift_metadata: { price: 1000 } }]);
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("事件回放 JSON"), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "提交事件回放" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("礼物元数据");
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it.each([
    [{ amount_cny: 0 }, "金额"], [{ amount_cny: 1.5 }, "金额"], [{ amount_cny: 1_000_001 }, "金额"],
    [{ text: " " }, "SC 内容"], [{ text: "猫".repeat(501) }, "500"],
    [{ start_at_ms: -1 }, "有效期"], [{ end_at_ms: 1000 }, "有效期"],
  ])("拒绝 SC 回放的非法字段 %#", async (patch, message) => {
    const onSubmit = vi.fn();
    const value = JSON.stringify([{ id: "sc", source: "recording", viewer: "小猫", kind: { type: "super_chat", text: "加油", amount_cny: 30, start_at_ms: 1000, end_at_ms: 2000, ...patch } }]);
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("事件回放 JSON"), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "提交事件回放" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(message);
    expect(screen.getByLabelText("事件回放 JSON")).toHaveValue(value);
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("聊天发送成功后清空文本并显示接收数量", async () => {
    const onSubmit = vi.fn().mockResolvedValue({ accepted: 1, duplicates: 0 });
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.change(screen.getByLabelText("聊天内容"), { target: { value: "晚上好" } });
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(onSubmit.mock.calls[0][0]).toMatchObject({ events: [{ source: "simulator", viewer: "小猫", kind: { type: "chat", text: "晚上好" } }] });
    expect(onSubmit.mock.calls[0][0].events[0].id).toEqual(expect.any(String));
    expect(await screen.findByRole("status")).toHaveTextContent("已接收 1 条事件");
    expect(screen.getByLabelText("聊天内容")).toHaveValue("");
  });

  it("礼物字段校验失败会保留输入", async () => {
    const onSubmit = vi.fn();
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole("button", { name: "礼物" }));
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.change(screen.getByLabelText("礼物名称"), { target: { value: "小鱼干" } });
    fireEvent.change(screen.getByLabelText("礼物数量"), { target: { value: "0" } });
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("礼物数量");
    expect(screen.getByLabelText("礼物名称")).toHaveValue("小鱼干");
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("提交失败时保留聊天内容", async () => {
    const onSubmit = vi.fn().mockResolvedValue(null);
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.change(screen.getByLabelText("聊天内容"), { target: { value: "请保留" } });
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1));
    expect(screen.getByLabelText("聊天内容")).toHaveValue("请保留");
  });

  it("回放保留原事件 ID 并显示纯重复反馈", async () => {
    const onSubmit = vi.fn().mockResolvedValue({ accepted: 0, duplicates: 1 });
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    const replay = JSON.stringify([{ id: "replay-original", source: "platform", viewer: "观众甲", kind: { type: "chat", text: "重放消息" } }]);
    fireEvent.change(screen.getByLabelText("事件回放 JSON"), { target: { value: replay } });
    fireEvent.click(screen.getByRole("button", { name: "提交事件回放" }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith({ events: [expect.objectContaining({ id: "replay-original", source: "platform" })] }));
    expect(await screen.findByRole("status")).toHaveTextContent("1 条事件已存在");
    expect(screen.getByLabelText("事件回放 JSON")).toHaveValue("");
  });

  it("反馈已持久保存但未安排回应的事件", async () => {
    const onSubmit = vi.fn().mockResolvedValue({ accepted: 0, duplicates: 0, persisted: 1, unscheduled: 1 });
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.change(screen.getByLabelText("聊天内容"), { target: { value: "保存但不播报" } });
    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));

    expect(await screen.findByRole("status")).toHaveTextContent("已持久保存 1 条事件，1 条未安排回应。");
  });

  it.each([
    ["非数组", JSON.stringify({ events: [] }), "JSON 数组"],
    ["空批次", "[]", "1 到 100"],
    ["过大批次", JSON.stringify(Array.from({ length: 101 }, (_, id) => ({ id: String(id), source: "x", viewer: "v", kind: { type: "chat", text: "x" } }))), "1 到 100"],
    ["空字段", JSON.stringify([{ id: "", source: "x", viewer: "v", kind: { type: "chat", text: "x" } }]), "不能为空"],
    ["错误计数", JSON.stringify([{ id: "1", source: "x", viewer: "v", kind: { type: "gift", name: "鱼", count: 0 } }]), "礼物数量"],
  ])("拒绝%s回放", async (_label, value, message) => {
    const onSubmit = vi.fn();
    render(<EventSimulator disabled={false} onSubmit={onSubmit} />);
    fireEvent.change(screen.getByLabelText("事件回放 JSON"), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "提交事件回放" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(message);
    expect(screen.getByLabelText("事件回放 JSON")).toHaveValue(value);
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
