import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createServerClient } from "../../services/server";
import { deferred, jsonResponse, serverStatus, speech } from "../../test/server-fixtures";
import { SpeechPanel } from "./SpeechPanel";

describe("状态刷新生命周期", () => {
  it("上一轮返回前不发起下一轮，卸载取消请求和计时器", async () => {
    vi.useFakeTimers();
    const first = deferred<Response>();
    let pendingSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      pendingSignal = init?.signal;
      return first.promise;
    });
    const client = createServerClient({ fetcher, timeoutMs: 30_000 });
    const view = render(<SpeechPanel client={client} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(1_000); });
    expect(fetcher).toHaveBeenCalledTimes(1);
    view.unmount();
    expect(pendingSignal?.aborted).toBe(true);
    await act(async () => {
      first.resolve(jsonResponse(serverStatus()));
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("按服务端回执从播放中更新为完成", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse(serverStatus({ speeches: [speech({ status: "playing" })] })))
      .mockImplementation(async () => jsonResponse(serverStatus({ speeches: [speech({ status: "completed" })] })));
    render(<SpeechPanel client={createServerClient({ fetcher })} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByText("播放中")).toBeVisible();
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });
    expect(screen.getByText("已完成")).toBeVisible();
  });

  it("停止之后忽略先前状态请求的迟到响应", async () => {
    vi.useFakeTimers();
    const late = deferred<Response>();
    let statusCalls = 0;
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url) => {
      if (String(url).endsWith("/api/stop")) return jsonResponse(serverStatus({ generation: 1, speeches: [speech({ status: "cancelled" })] }));
      statusCalls += 1;
      return statusCalls === 1 ? jsonResponse(serverStatus({ speeches: [speech({ status: "playing" })] })) : late.promise;
    });
    render(<SpeechPanel client={createServerClient({ fetcher })} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });
    fireEvent.click(screen.getByRole("button", { name: "停止全部播报" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByText("已取消")).toBeVisible();
    await act(async () => {
      late.resolve(jsonResponse(serverStatus({ speeches: [speech({ status: "playing" })] })));
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByText("已取消")).toBeVisible();
    expect(screen.queryByText("播放中")).not.toBeInTheDocument();
  });

  it("连接恢复后清除错误并允许继续提交", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockImplementation(async () => jsonResponse(serverStatus()));
    render(<SpeechPanel client={createServerClient({ fetcher })} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByRole("alert")).toHaveTextContent("连接");
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("播报文本"), { target: { value: "继续播报" } });
    expect(screen.getByRole("button", { name: "加入播报队列" })).toBeEnabled();
  });
});
