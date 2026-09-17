import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { TrainingSnapshot } from "@meowlive/contracts";
import type { ResourcesClient } from "../../services/server/resources";
import type { TrainingClient } from "../../services/server/training";
import { resourceSnapshot } from "../../test/resource-fixtures";
import { useTraining } from "./useTraining";

const snapshot: TrainingSnapshot = { enabled: true, busy: false, jobs: [], versions: [] };
const preset = { mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" };
function setup() {
  const client: TrainingClient = { modelStatus: vi.fn().mockResolvedValue({ supported: true, state: "loaded", message: "模型已启用" }), setModelsEnabled: vi.fn(), snapshot: vi.fn().mockResolvedValue(snapshot), preset: vi.fn().mockResolvedValue(preset), transcribe: vi.fn(), create: vi.fn(), cancel: vi.fn(), delete: vi.fn(), save: vi.fn(), activate: vi.fn(), audition: vi.fn(), measure: vi.fn() };
  const resources = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

describe("训练状态独立轮询", () => {
  it("资源和预设卡住时仍更新训练状态、刷新与取消操作", async () => {
    vi.useFakeTimers(); const d = setup();
    d.resources.getSnapshot = vi.fn(() => new Promise<never>(() => {}));
    d.client.preset = vi.fn(() => new Promise<never>(() => {}));
    const { result } = renderHook(() => useTraining(d.client, d.resources));
    await act(async () => {});
    expect(result.current.snapshot).toEqual(snapshot);
    vi.mocked(d.client.snapshot).mockResolvedValue({ ...snapshot, busy: true });
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    expect(result.current.snapshot?.busy).toBe(true);
    await act(async () => { await result.current.action(async () => {}); });
    expect(result.current.pending).toBe(false);
  });
  it("预设错误不屏蔽训练且仅在该查询恢复时清除", async () => {
    vi.useFakeTimers(); const d = setup();
    vi.mocked(d.client.preset).mockRejectedValueOnce(new Error("预设读取失败"));
    const { result } = renderHook(() => useTraining(d.client, d.resources));
    await act(async () => {});
    expect(result.current.snapshot).toEqual(snapshot);
    expect(result.current.error).toContain("预设读取失败");
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    expect(result.current.error).toBe("");
  });
  it("训练查询恢复清除旧错误但成功轮询保留操作失败", async () => {
    vi.useFakeTimers(); const d = setup();
    vi.mocked(d.client.snapshot).mockRejectedValueOnce(new Error("读取失败"));
    const { result } = renderHook(() => useTraining(d.client, d.resources));
    await act(async () => {}); expect(result.current.error).toBe("读取失败");
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    expect(result.current.error).toBe("");
    await act(async () => { await result.current.action(async () => { throw new Error("取消失败"); }); });
    await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
    expect(result.current.error).toBe("取消失败");
  });
  it("迟到查询不能覆盖操作后的新状态，卸载后不再轮询", async () => {
    vi.useFakeTimers(); const d = setup(); const old = deferred<TrainingSnapshot>();
    vi.mocked(d.client.snapshot).mockReturnValueOnce(old.promise);
    const { result, unmount } = renderHook(() => useTraining(d.client, d.resources));
    await act(async () => { await result.current.refresh(); });
    expect(result.current.snapshot?.busy).toBe(false);
    await act(async () => { old.resolve({ ...snapshot, busy: true }); });
    expect(result.current.snapshot?.busy).toBe(false);
    const calls = vi.mocked(d.client.snapshot).mock.calls.length;
    const signal = vi.mocked(d.client.snapshot).mock.calls[0]?.[0];
    unmount(); expect(signal?.aborted).toBe(true);
    await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
    expect(d.client.snapshot).toHaveBeenCalledTimes(calls);
  });
});

it("模型状态不可用或迟迟不返回，不阻塞任务查询和刷新", async () => {
  vi.useFakeTimers(); const deps = setup();
  vi.mocked(deps.client.modelStatus).mockRejectedValueOnce(new Error("TTS 未启动")).mockImplementation(() => new Promise(() => {}));
  const { result } = renderHook(() => useTraining(deps.client, deps.resources));
  await act(async () => {});
  expect(result.current.snapshot).toEqual(snapshot);
  expect(result.current.error).toBe("");
  expect(result.current.modelRuntime?.state).toBe("unavailable");
  await act(async () => { await result.current.refresh(); });
  expect(result.current.pending).toBe(false);
  expect(result.current.snapshot).toEqual(snapshot);
});

it("启停模型更新独立状态，迟到轮询不能覆盖，也不激活音色", async () => {
  vi.useFakeTimers(); const deps = setup();
  const old = deferred<{ supported: boolean; state: string; message: string }>();
  vi.mocked(deps.client.modelStatus).mockReturnValueOnce(old.promise).mockResolvedValue({ supported: true, state: "loaded", message: "已启用" });
  vi.mocked(deps.client.setModelsEnabled).mockResolvedValue({ supported: true, state: "loaded", message: "已启用" });
  const { result } = renderHook(() => useTraining(deps.client, deps.resources));
  await act(async () => { await result.current.setModelsEnabled(true); });
  expect(result.current.modelRuntime?.state).toBe("loaded");
  await act(async () => { old.resolve({ supported: true, state: "unloaded", message: "旧状态" }); });
  expect(result.current.modelRuntime?.state).toBe("loaded");
  expect(deps.client.activate).not.toHaveBeenCalled();
});
