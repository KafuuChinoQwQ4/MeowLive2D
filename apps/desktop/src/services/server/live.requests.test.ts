import { describe, expect, it, vi } from "vitest";
import { jsonResponse } from "../../test/server-fixtures";
import { liveSnapshot } from "../../test/live-fixtures";
import { createLiveClient } from "./live";

describe("直播连接 HTTP 请求", () => {
  it("从配置地址读取完整连接快照", async () => {
    const snapshot = liveSnapshot({
      phase: "connected",
      room_id: "24680",
      accepted_events: 23,
      duplicate_events: 4,
      rejected_events: 2,
      reconnect_attempts: 1,
    });
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshot));
    const client = createLiveClient({ baseUrl: "http://localhost:19700/", fetcher });

    await expect(client.getStatus()).resolves.toEqual(snapshot);
    expect(fetcher).toHaveBeenCalledWith("http://localhost:19700/api/live", expect.objectContaining({ method: "GET" }));
  });

  it.each([
    ["connect", "/api/live/connect"],
    ["disconnect", "/api/live/disconnect"],
  ] as const)("%s 调用对应控制路由并采用返回快照", async (method, path) => {
    const snapshot = liveSnapshot({ phase: method === "connect" ? "connecting" : "disconnecting" });
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshot));
    const client = createLiveClient({ fetcher });

    await expect(client[method]()).resolves.toEqual(snapshot);
    expect(fetcher).toHaveBeenCalledWith(`http://127.0.0.1:19600${path}`, expect.objectContaining({ method: "POST" }));
  });
});
