import { describe, expect, it, vi } from "vitest";
import { createTrainingClient, readTraining, readPreset } from "./training";
const snapshot = { enabled: true, busy: false, jobs: [], versions: [] };
describe("训练片段转写", () => {
  it("用单个音频和语言元数据请求转写", async () => {
    const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ text: "你好", language: "zh" })));
    const client = createTrainingClient({ baseUrl: "http://localhost:19600", fetcher });
    const file = new File(["wave"], "片段.wav", { type: "audio/wav" });
    expect(await client.transcribe(file, "zh")).toEqual({ text: "你好", language: "zh" });
    const [url, init] = fetcher.mock.calls[0]!;
    expect(url).toBe("http://localhost:19600/api/training/transcribe");
    expect(init.method).toBe("POST");
    expect(init.body.get("metadata")).toBe('{"language":"zh"}');
    expect(init.body.getAll("audio")).toEqual([file]);
  });
  it.each([
    { text: " ", language: "zh" }, { text: "字".repeat(501), language: "zh" },
    { text: "坏|文本", language: "zh" }, { text: "坏\n文本", language: "zh" },
    { text: "坏\u0085文本", language: "zh" }, { text: "你好", language: "unknown" },
    { text: "你好", language: ["zh"] }, { text: "Hello", language: "en" },
  ])("拒绝不能用于训练的转写结果 %j", async value => {
    const client = createTrainingClient({ fetcher: vi.fn().mockResolvedValue(new Response(JSON.stringify(value))) });
    await expect(client.transcribe(new File(["wave"], "clip.wav"), "zh")).rejects.toThrow("无效");
  });
  it("转写取消也覆盖忽略 AbortSignal 的传输", async () => {
    const client = createTrainingClient({ fetcher: vi.fn(() => new Promise<Response>(() => {})) });
    const ctrl = new AbortController();
    const pending = client.transcribe(new File(["wave"], "clip.wav"), "zh", ctrl.signal);
    ctrl.abort();
    await expect(pending).rejects.toMatchObject({ name: "AbortError" });
  });
  it("转写超时提供重试与手工填写提示", async () => {
    const client = createTrainingClient({ fetcher: vi.fn(() => new Promise<Response>(() => {})), timeoutMs: 10 });
    await expect(client.transcribe(new File(["wave"], "clip.wav"), "zh")).rejects.toThrow("请求超时，请重试提取文本或手工填写");
  });
});
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


it("删除训练版本使用 ID 并返回剩余快照，保留删除失败原因", async () => {
  const fetcher = vi.fn(async (url: RequestInfo | URL, init?: RequestInit) => {
    expect(url).toBe("http://localhost:19600/api/training/delete");
    expect(init?.method).toBe("POST");
    expect(JSON.parse(String(init?.body))).toEqual({ id: "version-1" });
    return new Response(JSON.stringify(snapshot));
  });
  const client = createTrainingClient({ baseUrl: "http://localhost:19600", fetcher });
  expect(await client.delete("version-1")).toEqual(snapshot);
  fetcher.mockResolvedValueOnce(new Response(JSON.stringify({ code: "training_failed", message: "记录已删除，文件清理失败，请重试删除" }), { status: 500 }));
  await expect(client.delete("version-1")).rejects.toThrow("文件清理失败");
});

describe("训练性能和模型内存开关", () => {
  const job = { id: "job", voice_id: "voice-1", name: "任务", status: "training", progress: 40, message: "训练中", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: null };
  const performance = { batch_size: 1, data_workers: 1, cpu_threads: 2, gpu_index: 0, low_memory: true };
  it("旧任务缺少性能参数时补默认值，拒绝越界的新参数", () => {
    expect(readTraining({ ...snapshot, jobs: [job] }).jobs[0].performance).toEqual(performance);
    for (const invalid of [{ batch_size: 0 }, { batch_size: 17 }, { data_workers: 9 }, { cpu_threads: 0 }, { gpu_index: 16 }, { low_memory: "true" }, { batch_size: 1.5 }]) {
      expect(() => readTraining({ ...snapshot, jobs: [{ ...job, performance: { ...performance, ...invalid } }] })).toThrow("无效");
    }
  });
  it("模型状态与独立开关走相同端点，不发送音色激活请求", async () => {
    const fetcher = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => new Response(JSON.stringify({ supported: true, state: init?.method === "POST" ? "loaded" : "unloaded", message: "就绪" })));
    const client = createTrainingClient({ baseUrl: "http://localhost:19600", fetcher });
    expect((await client.modelStatus()).state).toBe("unloaded");
    expect((await client.setModelsEnabled(true)).state).toBe("loaded");
    expect(fetcher.mock.calls.map(([url]) => url)).toEqual(["http://localhost:19600/api/training/models", "http://localhost:19600/api/training/models"]);
    expect(JSON.parse(String(fetcher.mock.calls[1][1]?.body))).toEqual({ enabled: true });
  });
  it.each([{ supported: true, state: "unknown", message: "未知" }, { supported: "true", state: "loaded", message: "错误" }, { supported: false, state: "loaded", message: "错误" }])("拒绝无效模型状态 %j", async value => {
    const client = createTrainingClient({ fetcher: vi.fn().mockResolvedValue(new Response(JSON.stringify(value))) });
    await expect(client.modelStatus()).rejects.toThrow("无效");
  });
});
