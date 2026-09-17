import { describe, expect, it, vi } from "vitest";
import { character, pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import { jsonResponse } from "../../test/server-fixtures";
import { createResourceClient } from "./resources";

describe("资源 HTTP 请求", () => {
  it("读取首次安装尚未选择音色的空资源快照", async () => {
    const snapshot = resourceSnapshot({ voices: [], characters: [], active_voice_id: "", active_character_id: null, default_voice_available: false });
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshot));

    await expect(createResourceClient({ fetcher }).getSnapshot()).resolves.toEqual(snapshot);
  });

  it("读取资源快照", async () => {
    const snapshot = resourceSnapshot();
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshot));
    const client = createResourceClient({ baseUrl: "http://localhost:19700/", fetcher });

    await expect(client.getSnapshot()).resolves.toEqual(snapshot);
    expect(fetcher).toHaveBeenCalledWith("http://localhost:19700/api/resources", expect.objectContaining({ method: "GET" }));
  });

  it("以 metadata 和原始 WAV 文件上传音色", async () => {
    const audio = pcm16Wav();
    const metadata = { name: "温柔旁白", language: "zh", reference_text: "你好" };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(resourceSnapshot(), 201));

    await createResourceClient({ fetcher }).createVoice(metadata, audio);

    const init = fetcher.mock.calls[0]?.[1];
    expect(fetcher.mock.calls[0]?.[0]).toBe("http://127.0.0.1:19600/api/voices");
    expect(init?.method).toBe("POST");
    expect(init?.headers).toBeUndefined();
    const body = init?.body as FormData;
    expect(JSON.parse(String(body.get("metadata")))).toEqual(metadata);
    expect(body.get("audio")).toBe(audio);
  });

  it.each([
    ["selectVoice", "/api/voices/select", { id: "voice-1" }],
    ["deleteVoice", "/api/voices/delete", { id: "voice-1" }],
    ["deleteCharacter", "/api/characters/delete", { id: "character-1" }],
    ["selectCharacter", "/api/characters/select", { id: "character-1" }],
    ["previewCharacter", "/api/characters/preview", { character_id: "character-1", intent: "挥手" }],
  ] as const)("%s 发送对应 JSON 请求", async (method, path, body) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(resourceSnapshot()));
    const client = createResourceClient({ fetcher });

    await client[method](body as never);

    expect(fetcher).toHaveBeenCalledWith(`http://127.0.0.1:19600${path}`, expect.objectContaining({
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    }));
  });

  it("保存完整角色编辑请求", async () => {
    const body = {
      id: null,
      name: "新角色",
      model_id: "model-1",
      voice_id: "voice-1",
      mouth_parameter: "ParamMouthOpenY",
      mappings: [{ intent: "挥手", hotkey_id: "wave", fallback_hotkey_id: null, validated: false }],
    };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(resourceSnapshot()));

    await createResourceClient({ fetcher }).saveCharacter(body);

    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/characters/save", expect.objectContaining({ body: JSON.stringify(body) }));
  });

  it("向桌面资源路由发送联合操作并读取结果", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ type: "models", models: [{ id: "model-1", name: "Cat" }] }));

    await expect(createResourceClient({ fetcher }).desktop({ type: "list_models" })).resolves.toEqual({
      type: "models", models: [{ id: "model-1", name: "Cat" }],
    });
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/desktop/resources", expect.objectContaining({
      method: "POST", body: JSON.stringify({ type: "list_models" }),
    }));
  });

  it("读取删除音色后待重新绑定的角色", async () => {
    const snapshot = resourceSnapshot({ voices: [], characters: [character({ voice_id: "" })], active_voice_id: "", active_character_id: null });
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshot));
    await expect(createResourceClient({ fetcher }).getSnapshot()).resolves.toEqual(snapshot);
  });

  it("读取安装模型列表并校验删除结果对应所选模型", async () => {
    const id = "a".repeat(64);
    const listed = { type: "imported_models", models: [{ id, name: "猫咪", model_id: null }] };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValueOnce(jsonResponse(listed))
      .mockResolvedValueOnce(jsonResponse({ type: "model_deleted", id, restart_required: true }))
      .mockResolvedValueOnce(jsonResponse({ type: "model_deleted", id: "b".repeat(64), restart_required: true }));
    const client = createResourceClient({ fetcher });
    await expect(client.desktop({ type: "list_imported_models" })).resolves.toEqual(listed);
    await expect(client.desktop({ type: "delete_imported_model", id })).resolves.toMatchObject({ type: "model_deleted", id });
    await expect(client.desktop({ type: "delete_imported_model", id })).rejects.toThrow("桌面资源结果与请求不匹配");
  });
});
