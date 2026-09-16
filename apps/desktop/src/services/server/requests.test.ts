import { describe, expect, it, vi } from "vitest";
import { jsonResponse, serverStatus, speech } from "../../test/server-fixtures";
import { createServerClient } from "./index";

describe("主服务 HTTP 请求", () => {
  it("从配置地址获取状态并保留协议字段", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus()));
    const client = createServerClient({ baseUrl: "http://localhost:19700/", fetcher });

    await expect(client.getStatus()).resolves.toMatchObject({
      session_id: "session-1", bridge_connected: true, speeches: [],
    });
    expect(fetcher).toHaveBeenCalledWith("http://localhost:19700/api/status", expect.objectContaining({ method: "GET" }));
  });

  it("向 speech 接口发送文本与声音并接受 202 任务快照", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(speech(), 202));
    const client = createServerClient({ fetcher });

    await expect(client.submitSpeech({ text: "欢迎来到直播间", voice_id: "default" })).resolves.toMatchObject({
      id: "speech-1", status: "queued",
    });
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/speech", expect.objectContaining({
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text: "欢迎来到直播间", voice_id: "default" }),
    }));
  });

  it("停止后返回服务端提升过的代次", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus({ generation: 1 })));
    const client = createServerClient({ fetcher });

    await expect(client.stop()).resolves.toMatchObject({ generation: 1 });
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/stop", expect.objectContaining({ method: "POST" }));
  });
});
