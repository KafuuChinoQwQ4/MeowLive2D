import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EventSimulator } from "./EventSimulator";

describe("直播事件模拟与回放", () => {
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
