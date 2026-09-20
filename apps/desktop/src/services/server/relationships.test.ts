import { expect, it, vi } from "vitest";
import { createRelationshipClient } from "./relationships";
it("refuses forged nested relationship data", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(
      new Response(
        JSON.stringify({ relationships: [{ source: null }], degraded: false }),
      ),
    );
  await expect(
    createRelationshipClient({ fetcher }).list("v", 1),
  ).rejects.toThrow();
});
it("uses expected version and exact retry key for graph edits", async () => {
  const fact = {
    id: "r",
    version: 2,
    source: { kind: "viewer", id: "v" },
    target: { kind: "topic", id: "cats" },
    kind: "shared_interest",
    confirmation: "confirmed",
    evidence: [],
    expires_at_ms: null,
    deleted: false,
  };
  const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify(fact)));
  const r = {
    action: "confirm",
    expected_version: 1,
    reason: "证据",
    request_key: "same",
  };
  await createRelationshipClient({ baseUrl: "http://x", fetcher }).change(
    "r",
    r,
  );
  expect(fetcher).toHaveBeenCalledWith(
    "http://x/api/admin/relationships/r",
    expect.objectContaining({ body: JSON.stringify(r) }),
  );
});
