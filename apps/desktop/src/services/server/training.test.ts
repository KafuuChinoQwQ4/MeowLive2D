import { describe, expect, it, vi } from "vitest";
import { createTrainingClient, readTraining, readPreset } from "./training";
const snapshot = { enabled: true, busy: false, jobs: [], versions: [] };
describe("训练服务边界", () => {
  it("拒绝未知阶段和不属于完成任务的模型版本", () => {
    expect(readTraining(snapshot)).toEqual(snapshot);
    expect(() => readTraining({ ...snapshot, versions: [{ id: "wrong" }] })).toThrow("无效");
    expect(() => readTraining({ ...snapshot, jobs: [{}] })).toThrow("无效");
  });
  it("没有实测证据不能标记离线已验证", () => {
    const value = { mode: "local", model: "local", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" };
    expect(readPreset(value).verified).toBe(false);
    expect(() => readPreset({ ...value, verified: true })).toThrow("无效");
  });
  it("超时和取消也覆盖不响应 AbortSignal 的传输", async () => {
    const client = createTrainingClient({ fetcher: vi.fn(() => new Promise<Response>(() => {})), timeoutMs: 10 });
    await expect(client.snapshot()).rejects.toThrow("请求超时");
    const ctrl = new AbortController();
    const pending = client.snapshot(ctrl.signal); ctrl.abort();
    await expect(pending).rejects.toMatchObject({ name: "AbortError" });
  });
  it("试听拒绝非 WAV 成功响应并保留服务器错误", async () => {
    const client = createTrainingClient({ fetcher: vi.fn().mockResolvedValue(new Response("{}", { headers: { "content-type": "application/json" } })) });
    await expect(client.audition("version", "你好")).rejects.toThrow("无效");
    const failed = createTrainingClient({ fetcher: vi.fn().mockResolvedValue(new Response(JSON.stringify({ code: "gpu_busy", message: "训练正在进行" }), { status: 409 })) });
    await expect(failed.activate("version")).rejects.toThrow("训练正在进行");
  });
});

describe("训练契约回归", () => {
  it("保存音色请求返回持久化标记并拒绝未试听却已保存的版本", async () => {
    const job = { id: "version-1", voice_id: "voice-1", name: "温柔音色", status: "completed", progress: 100, message: "训练完成", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: "version-1" };
    const version = { id: "version-1", job_id: "version-1", voice_id: "voice-1", name: "温柔音色", engine: "gpt-sovits", model_version: "v2", auditioned: true, saved: true, active: false, available: true, created_at_ms: 1 };
    const value = { ...snapshot, jobs: [job], versions: [version] };
    const fetcher = vi.fn(async (url: RequestInfo | URL, init?: RequestInit) => {
      expect(url).toBe("http://localhost:19600/api/training/save");
      expect(init?.method).toBe("POST");
      expect(JSON.parse(String(init?.body))).toEqual({ id: "version-1" });
      return new Response(JSON.stringify(value));
    });
    const client = createTrainingClient({ baseUrl: "http://localhost:19600", fetcher });
    expect((await client.save("version-1")).versions[0].saved).toBe(true);
    expect(() => readTraining({ ...value, versions: [{ ...version, auditioned: false }] })).toThrow("无效");
    expect(() => readTraining({ ...value, versions: [{ ...version, saved: "true" }] })).toThrow("无效");
  });
  const value = { mode: "local", model: "local", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" };
  it("接受服务端合法多行回复并拒绝其余控制字符和超长回复", () => {
    const measurement = { measured_at_ms: 1, llm_ms: 1, tts_ms: 1, total_ms: 2, gpu_name: "RTX", gpu_total_mib: 6141, gpu_peak_used_mib: 2000, voice_id: "voice-1", reply: "你好\n欢迎\t光临\r直播间", passed: true };
    expect(readPreset({ ...value, verified: true, measurement }).measurement?.reply).toBe(measurement.reply);
    for (const reply of [" ", "错误\u0000文本", "错误\u0085文本", "字".repeat(501)]) {
      expect(() => readPreset({ ...value, measurement: { ...measurement, reply } })).toThrow("无效");
    }
  });
  it("拒绝数组伪装的阶段和预设模式", () => {
    const job = { id: "job", voice_id: "voice-1", name: "任务", status: ["training"], progress: 40, message: "训练中", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: null };
    expect(() => readTraining({ ...snapshot, jobs: [job] })).toThrow("无效");
    expect(() => readPreset({ ...value, mode: ["local"] })).toThrow("无效");
  });
});
