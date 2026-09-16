import { describe, expect, it, vi } from "vitest";
import { resourceSnapshot, voice } from "../../test/resource-fixtures";
import { deferred, jsonResponse } from "../../test/server-fixtures";
import { createResourceClient } from "./resources";

describe("资源响应校验", () => {
  it.each([
    ["默认音色混入上传列表", resourceSnapshot({ voices: [voice({ id: "default" })] })],
    ["未知活动音色", resourceSnapshot({ active_voice_id: "missing" })],
    ["越界音色列表", resourceSnapshot({ voices: Array.from({ length: 65 }, (_, index) => voice({ id: `voice-${index}` })) })],
    ["无效音频时长", resourceSnapshot({ voices: [voice({ duration_ms: 2_999 })] })],
    ["缺失文件却没有原因", resourceSnapshot({ voices: [voice({ available: false, error: null })] })],
    ["重复角色意图", resourceSnapshot({ characters: [{ ...resourceSnapshot().characters[0]!, mappings: [
      { intent: "挥手", hotkey_id: "one", fallback_hotkey_id: null, validated: false },
      { intent: "挥手", hotkey_id: "two", fallback_hotkey_id: null, validated: false },
    ] }] })],
    ["未知活动角色", resourceSnapshot({ active_character_id: "missing" })],
    ["不安全的音色 ID", resourceSnapshot({ voices: [voice({ id: "../voice" })], active_voice_id: "default" })],
    ["越界的无符号整数", resourceSnapshot({ voices: [voice({ duration_ms: 0x1_0000_0000 })] })],
  ])("拒绝%s", async (_label, value) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
    await expect(createResourceClient({ fetcher }).getSnapshot()).rejects.toMatchObject({ code: "invalid_response" });
  });

  it.each([
    { type: "models", models: Array.from({ length: 257 }, (_, index) => ({ id: `m-${index}`, name: "M" })) },
    { type: "hotkeys", model_id: "model-1", hotkeys: [{ id: "", name: "Bad" }] },
    { type: "model_imported", model_name: "Cat", model_file: "/private/model.json", files: 1, bytes: 10, restart_required: true },
    { type: "unknown" },
  ])("拒绝损坏或泄露路径的桌面结果", async (value) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
    await expect(createResourceClient({ fetcher }).desktop({ type: "list_models" })).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("拒绝与桌面请求不匹配的有效结果", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ type: "models", models: [] }));
    await expect(createResourceClient({ fetcher }).desktop({ type: "import_model" }))
      .rejects.toMatchObject({ code: "invalid_response" });
  });
});

describe("资源请求失败与取消", () => {
  it("保留服务端缺失文件原因", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ code: "voice_file_missing", message: "音色参考文件缺失，请重新上传" }, 409));
    await expect(createResourceClient({ fetcher }).selectVoice({ id: "voice-1" })).rejects.toMatchObject({
      code: "voice_file_missing", message: "音色参考文件缺失，请重新上传", status: 409,
    });
  });

  it("取消会中断桌面资源请求", async () => {
    const pending = deferred<Response>();
    let requestSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      requestSignal = init?.signal;
      return pending.promise;
    });
    const controller = new AbortController();
    const request = createResourceClient({ fetcher }).desktop({ type: "list_models" }, controller.signal);
    controller.abort();

    await expect(request).rejects.toMatchObject({ name: "AbortError" });
    expect(requestSignal?.aborted).toBe(true);
  });

  it("超时会中止无响应请求", async () => {
    vi.useFakeTimers();
    let signal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      signal = init?.signal;
      return new Promise<Response>(() => undefined);
    });
    const request = createResourceClient({ fetcher, timeoutMs: 100 }).getSnapshot();
    const assertion = expect(request).rejects.toMatchObject({ code: "request_timeout" });

    await Promise.all([assertion, vi.advanceTimersByTimeAsync(100)]);
    expect(signal?.aborted).toBe(true);
  });
});
