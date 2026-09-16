import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createServerClient } from "../../services/server";
import { deferred, jsonResponse, serverStatus, speech } from "../../test/server-fixtures";
import { SpeechPanel } from "./SpeechPanel";

describe("播报操作场景", () => {
  it("活动任务可以停止，随后使用服务端取消状态", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url) => String(url).endsWith("/api/stop")
      ? jsonResponse(serverStatus({ generation: 1, speeches: [speech({ status: "cancelled" })] }))
      : jsonResponse(serverStatus({ speeches: [speech({ status: "playing" })] })));
    render(<SpeechPanel client={createServerClient({ fetcher })} pollIntervalMs={10_000} />);
    await screen.findByText("播放中");
    fireEvent.click(screen.getByRole("button", { name: "停止全部播报" }));
    await screen.findByText("已取消");
    expect(screen.getByRole("button", { name: "停止全部播报" })).toBeDisabled();
  });

  it("提交失败保留输入和服务端原因，不产生虚假任务", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url) => String(url).endsWith("/api/speech")
      ? jsonResponse({ code: "queue_full", message: "播报队列已满" }, 429)
      : jsonResponse(serverStatus()));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "你好" } });
    fireEvent.click(screen.getByRole("button", { name: "加入播报队列" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("播报队列已满");
    expect(screen.getByLabelText("播报文本")).toHaveValue("你好");
    expect(screen.queryByRole("listitem")).not.toBeInTheDocument();
  });

  it("请求未完成时阻止重复提交", async () => {
    const pending = deferred<Response>();
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url) => String(url).endsWith("/api/speech") ? pending.promise : jsonResponse(serverStatus()));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await screen.findByText("桌面执行端已连接");
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "你好" } });
    fireEvent.click(screen.getByRole("button", { name: "加入播报队列" }));
    expect(screen.getByRole("button", { name: "正在提交…" })).toBeDisabled();
    expect(fetcher.mock.calls.filter(([url]) => String(url).endsWith("/api/speech"))).toHaveLength(1);
    pending.resolve(jsonResponse(speech(), 202));
    await waitFor(() => expect(screen.queryByText("正在提交…")).not.toBeInTheDocument());
  });

  it("主服务不可达时呈现连接错误并禁止提交", async () => {
    const fetcher = vi.fn<typeof fetch>().mockRejectedValue(new TypeError("Failed to fetch"));
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("连接");
    expect(screen.getByRole("button", { name: "加入播报队列" })).toBeDisabled();
  });
});
