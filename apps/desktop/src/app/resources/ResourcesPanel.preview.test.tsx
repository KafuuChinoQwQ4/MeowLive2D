import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";
import { resourceSnapshot } from "../../test/resource-fixtures";
import { deferred, jsonResponse, serverStatus, speech } from "../../test/server-fixtures";
import { ResourcesPanel } from "./ResourcesPanel";

function setup(fetcher: typeof fetch) {
  const resourceClient = {
    getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()),
  } as unknown as ResourcesClient;
  return render(<ResourcesPanel mode="voices" resourceClient={resourceClient} speechClient={createServerClient({ fetcher })} />);
}

async function preview() {
  await act(async () => {});
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "试听" })));
}

describe("音色试听进度", () => {
  it.each([
    ["failed", "Windows audio device stream failed", "试听失败"],
    ["completed", null, "试听已完成"],
  ] as const)("将已排队试听更新为服务端的 %s，并停止状态查询", async (status, error, label) => {
    vi.useFakeTimers();
    let terminal = false;
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async url => {
      if (String(url).endsWith("/api/speech")) return jsonResponse(speech(), 202);
      return jsonResponse(serverStatus({ speeches: [speech(terminal ? { status, error } : { status: "synthesizing" })] }));
    });
    setup(fetcher);
    await preview();
    expect(screen.getByText(/试听已加入播报队列/)).toBeVisible();
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(screen.getByText(/试听正在合成/)).toBeVisible();
    terminal = true;
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(screen.getByText(new RegExp(label))).toBeVisible();
    if (error) expect(screen.getByText(new RegExp(error))).toBeVisible();
    const calls = fetcher.mock.calls.length;
    await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
    expect(fetcher.mock.calls).toHaveLength(calls);
  });

  it.each([["unknown", "试听结果未知"], ["cancelled", "试听已取消"]] as const)("执行端断开时显示真实 %s 结果及断开原因", async (status, label) => {
    vi.useFakeTimers();
    setup(vi.fn<typeof fetch>().mockImplementation(async url => String(url).endsWith("/api/speech")
      ? jsonResponse(speech(), 202)
      : jsonResponse(serverStatus({ bridge_connected: false, speeches: [speech({ status })] }))));
    await preview();
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(screen.getByText(new RegExp(label))).toBeVisible();
    expect(screen.getByRole("alert")).toHaveTextContent(/Windows 执行端.*断开/);
    expect(screen.queryByText(/试听已完成/)).not.toBeInTheDocument();
  });

  it("主服务丢失试听记录后显示结果未知并停止查询", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async url => String(url).endsWith("/api/speech")
      ? jsonResponse(speech(), 202)
      : jsonResponse(serverStatus({ speeches: [speech({ generation: 1, status: "completed" })] })));
    setup(fetcher);
    await preview();
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(screen.getByText(/试听结果未知/)).toBeVisible();
    expect(screen.getByRole("alert")).toHaveTextContent(/试听记录已不可用/);
    expect(screen.queryByText(/试听已完成/)).not.toBeInTheDocument();
    const calls = fetcher.mock.calls.length;
    await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
    expect(fetcher.mock.calls).toHaveLength(calls);
  });

  it("查询失败显示原因，恢复后继续跟踪同一试听", async () => {
    vi.useFakeTimers();
    let offline = true;
    setup(vi.fn<typeof fetch>().mockImplementation(async url => {
      if (String(url).endsWith("/api/speech")) return jsonResponse(speech(), 202);
      if (offline) throw new TypeError("network offline");
      return jsonResponse(serverStatus({ speeches: [speech({ status: "completed" })] }));
    }));
    await preview();
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(screen.getByRole("alert")).toHaveTextContent(/试听状态.*无法连接主服务/);
    offline = false;
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(screen.getByText(/试听已完成/)).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("再次试听取消旧查询，迟到结果不能覆盖新试听", async () => {
    vi.useFakeTimers();
    const old = deferred<Response>();
    const next = deferred<Response>();
    let submissions = 0;
    let queries = 0;
    let oldSignal: AbortSignal | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url, init) => {
      if (String(url).endsWith("/api/speech")) return jsonResponse(speech({ id: `preview-${++submissions}` }), 202);
      queries++;
      if (queries === 1) { oldSignal = init?.signal ?? undefined; return old.promise; }
      return next.promise;
    });
    setup(fetcher);
    await preview();
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    await preview();
    expect(oldSignal?.aborted).toBe(true);
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    await act(async () => { next.resolve(jsonResponse(serverStatus({ speeches: [speech({ id: "preview-2", status: "completed" })] }))); });
    expect(screen.getByText(/试听已完成/)).toBeVisible();
    await act(async () => { old.resolve(jsonResponse(serverStatus({ speeches: [speech({ id: "preview-1", status: "failed", error: "旧试听失败" })] }))); });
    expect(screen.getByText(/试听已完成/)).toBeVisible();
    expect(screen.queryByText(/旧试听失败/)).not.toBeInTheDocument();
  });

  it("卸载取消查询，迟到响应不再发起轮询", async () => {
    vi.useFakeTimers();
    const pending = deferred<Response>();
    let signal: AbortSignal | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url, init) => {
      if (String(url).endsWith("/api/speech")) return jsonResponse(speech(), 202);
      signal = init?.signal ?? undefined;
      return pending.promise;
    });
    const view = setup(fetcher);
    await preview();
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    view.unmount();
    expect(signal?.aborted).toBe(true);
    const calls = fetcher.mock.calls.length;
    await act(async () => {
      pending.resolve(jsonResponse(serverStatus({ speeches: [speech({ status: "playing" })] })));
      await vi.advanceTimersByTimeAsync(4000);
    });
    expect(fetcher.mock.calls).toHaveLength(calls);
  });
});
