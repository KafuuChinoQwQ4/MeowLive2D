import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { createServerClient } from "../../services/server";
import { jsonResponse, serverStatus, speech } from "../../test/server-fixtures";
import { SpeechPanel } from "./SpeechPanel";

describe("人工播报表单", () => {
  it("使用中文标签和默认声音提交文本，显示排队任务", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url) => {
      if (String(url).endsWith("/api/speech")) return jsonResponse(speech(), 202);
      return jsonResponse(serverStatus());
    });
    const user = userEvent.setup();
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    expect(screen.getByLabelText("声音 ID")).toHaveValue("active");
    await user.type(screen.getByLabelText("播报文本"), "欢迎来到直播间");
    await user.click(screen.getByRole("button", { name: "加入播报队列" }));

    await screen.findByText("排队中");
    expect(screen.getByRole("list", { name: "播报任务" })).toHaveTextContent("欢迎来到直播间");
    expect(screen.getByLabelText("播报文本")).toHaveValue("");
    const request = fetcher.mock.calls.find(([url]) => String(url).endsWith("/api/speech"));
    expect(JSON.parse(String(request?.[1]?.body))).toEqual({ text: "欢迎来到直播间", voice_id: "active" });
  });

  it("空白文本和空白声音不能提交", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus()));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    const submit = screen.getByRole("button", { name: "加入播报队列" });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "你好" } });
    fireEvent.change(screen.getByLabelText("声音 ID"), { target: { value: "  " } });
    expect(submit).toBeDisabled();
  });

  it("没有桌面执行端时解释不可用原因", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus({ bridge_connected: false })));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端未连接");
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "你好" } });
    expect(screen.getByRole("button", { name: "加入播报队列" })).toBeDisabled();
    expect(screen.getByText(/启动桌面执行客户端/)).toBeVisible();
  });

  it("按 Unicode 字符计数允许 500 字并阻止超长文本", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => jsonResponse(serverStatus()));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "🐱".repeat(500) } });
    expect(screen.getByRole("button", { name: "加入播报队列" })).toBeEnabled();
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "🐱".repeat(501) } });
    expect(screen.getByRole("button", { name: "加入播报队列" })).toBeDisabled();
    expect(screen.getByText(/最多 500 字/)).toBeVisible();
  });

  it("声音 ID 只接受配置支持的字符", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => jsonResponse(serverStatus()));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "你好" } });
    fireEvent.change(screen.getByLabelText("声音 ID"), { target: { value: "bad/voice" } });
    expect(screen.getByRole("button", { name: "加入播报队列" })).toBeDisabled();
  });

  it("任务失败原因可见，终态任务不能触发停止", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus({ speeches: [speech({ status: "failed", error: "未配置参考音频" })] })));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("失败");
    expect(screen.getByText("未配置参考音频")).toBeVisible();
    expect(screen.getByRole("button", { name: "停止全部播报" })).toBeDisabled();
  });

  it("任务历史最多展示最近 50 条", async () => {
    const speeches = Array.from({ length: 60 }, (_, index) => speech({ id: `task-${index}`, text: `历史任务 ${index}`, status: "completed" }));
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus({ speeches })));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await waitFor(() => expect(screen.getAllByRole("listitem")).toHaveLength(50));
    expect(screen.queryByText("历史任务 0")).not.toBeInTheDocument();
    expect(screen.getByText("历史任务 59")).toBeVisible();
  });

  it("裁剪终态历史时保留较早的活动任务和停止能力", async () => {
    const speeches = [speech({ id: "active", text: "正在播放的旧任务", status: "playing" }),
      ...Array.from({ length: 60 }, (_, index) => speech({ id: `done-${index}`, status: "completed" }))];
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus({ speeches })));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    expect(screen.getByText("正在播放的旧任务")).toBeVisible();
    expect(screen.getByRole("button", { name: "停止全部播报" })).toBeEnabled();
  });
});
