import { describe, expect, it, vi } from "vitest";
import { createViewerClient } from "./viewers";
import { jsonResponse } from "../../test/server-fixtures";

describe("观众记录服务", () => {
  it("读取分页观众和事件，并保留平台原始礼物字段", async () => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse({ scope_id: "default", offset: 0, viewers: [{ viewer_id: "viewer-1", current_alias: "小猫", alias_observed_at_ms: 1, identities: [], aliases: [{ alias: "旧昵称", first_seen_at_ms: 1, last_seen_at_ms: 2 }] }] }))
      .mockResolvedValueOnce(jsonResponse({ scope_id: "default", offset: 0, unconfirmed_events: 2, events: [{ event_id: "event-1", source: "bilibili", session_id: "session-1", viewer_id: "viewer-1", viewer: "小猫", occurred_at_ms: 1, received_at_ms: 2, kind: { type: "gift", name: "花", count: 3 }, gift_metadata: { price: 1000, paid: true, medal_level: 12, guard_level: 3 } }] }));
    const client = createViewerClient({ baseUrl: "http://127.0.0.1:19994", fetcher });

    const [viewers, events] = await Promise.all([client.listViewers(0), client.listEvents(0)]);

    expect(viewers.viewers[0].aliases[0].alias).toBe("旧昵称");
    expect(events.unconfirmed_events).toBe(2);
    expect(events.events[0].gift_metadata?.price).toBe(1000);
    expect(fetcher).toHaveBeenNthCalledWith(1, "http://127.0.0.1:19994/api/admin/viewers?limit=50&offset=0", expect.objectContaining({ method: "GET" }));
  });
});

it("rejects malformed nested viewer identities", async () => {
 const fetcher=vi.fn().mockResolvedValue(new Response(JSON.stringify({scope_id:"s",offset:0,viewers:[{viewer_id:"v",aliases:[],identities:[null]}]})));
 await expect(createViewerClient({fetcher}).listViewers(0)).rejects.toThrow();
});
