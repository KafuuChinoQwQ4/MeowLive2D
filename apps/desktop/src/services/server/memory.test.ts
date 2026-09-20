import { expect, it, vi } from "vitest";
import { createMemoryClient } from "./memory";
it("rejects malformed memory evidence", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(
      new Response(
        JSON.stringify({ memories: [{ id: "m", evidence: [null] }] }),
      ),
    );
  await expect(createMemoryClient({ fetcher }).list("v")).rejects.toThrow();
});
it("sends expected version and idempotency key", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(new Response(JSON.stringify({ updated: true })));
  const r = {
    request_key: "same-key",
    reason: "纠错",
    expected_version: 3,
    operation: "delete",
    value: null,
    frozen: null,
  };
  await createMemoryClient({ baseUrl: "http://local", fetcher }).mutate(
    "v",
    "m",
    r,
  );
  expect(fetcher).toHaveBeenCalledWith(
    "http://local/api/admin/viewers/v/memories/m",
    expect.objectContaining({ body: JSON.stringify(r) }),
  );
});
it("recovery uses caller key and explicit reason", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(new Response(JSON.stringify({ updated: true })));
  const r = { request_key: "same", reason: "修复模型配置后恢复" };
  await createMemoryClient({ baseUrl: "http://x", fetcher }).retry(r);
  expect(fetcher).toHaveBeenCalledWith(
    "http://x/api/admin/memories/retry",
    expect.objectContaining({ body: JSON.stringify(r) }),
  );
});
