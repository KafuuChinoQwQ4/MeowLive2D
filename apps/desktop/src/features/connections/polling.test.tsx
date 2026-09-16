import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StrictMode } from "react";
import { describe, expect, it, vi } from "vitest";
import { createLiveClient } from "../../services/server/live";
import { liveSnapshot } from "../../test/live-fixtures";
import { deferred, jsonResponse } from "../../test/server-fixtures";
import { ConnectionPanel } from "./ConnectionPanel";

describe("直播连接状态刷新", () => {
  it("严格模式重新挂载后只保留一条轮询链", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(liveSnapshot()));
    render(<StrictMode><ConnectionPanel client={createLiveClient({ fetcher })} pollIntervalMs={100} /></StrictMode>);

    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    const callsAfterMount = fetcher.mock.calls.length;
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });

    expect(fetcher).toHaveBeenCalledTimes(callsAfterMount + 1);
  });

  it("上一轮返回前不重叠轮询，卸载时取消请求和计时器", async () => {
    vi.useFakeTimers();
    const first = deferred<Response>();
    let pendingSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      pendingSignal = init?.signal;
      return first.promise;
    });
    const view = render(<ConnectionPanel client={createLiveClient({ fetcher, timeoutMs: 30_000 })} pollIntervalMs={100} />);

    await act(async () => { await vi.advanceTimersByTimeAsync(1_000); });
    expect(fetcher).toHaveBeenCalledTimes(1);
    view.unmount();
    expect(pendingSignal?.aborted).toBe(true);
    await act(async () => {
      first.resolve(jsonResponse(liveSnapshot()));
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("连接操作响应不会被先前轮询的迟到结果覆盖", async () => {
    vi.useFakeTimers();
    const late = deferred<Response>();
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url) => {
      if (String(url).endsWith("/api/live/connect")) {
        return jsonResponse(liveSnapshot({ phase: "connecting" }));
      }
      return fetcher.mock.calls.length === 1
        ? jsonResponse(liveSnapshot({ phase: "disconnected" }))
        : late.promise;
    });
    render(<ConnectionPanel client={createLiveClient({ fetcher })} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });

    fireEvent.click(screen.getByRole("button", { name: "连接直播间" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByRole("heading", { name: "正在连接直播间…" })).toBeVisible();

    await act(async () => {
      late.resolve(jsonResponse(liveSnapshot({ phase: "disconnected" })));
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByRole("heading", { name: "正在连接直播间…" })).toBeVisible();
  });

  it("卸载时也取消正在进行的连接操作", async () => {
    const action = deferred<Response>();
    let actionSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url, init) => {
      if (String(url).endsWith("/api/live/connect")) {
        actionSignal = init?.signal;
        return action.promise;
      }
      return jsonResponse(liveSnapshot());
    });
    const view = render(<ConnectionPanel client={createLiveClient({ fetcher })} />);
    const connect = screen.getByRole("button", { name: "连接直播间" });
    await waitFor(() => expect(connect).toBeEnabled());
    fireEvent.click(connect);

    view.unmount();

    expect(actionSignal?.aborted).toBe(true);
  });
});
