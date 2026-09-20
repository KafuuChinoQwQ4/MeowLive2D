import { expect, it, vi } from "vitest";
import { createCompanionshipClient } from "./companionship";
it("encodes real IDs and preserves caller idempotency key", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(new Response(JSON.stringify({ record_id: "ledger" })));
  const c = createCompanionshipClient({ baseUrl: "http://local", fetcher });
  const r = { request_key: "retry-key", reason: "核实", delta_milli: -200 };
  await c.adjust("a/b", r);
  expect(fetcher).toHaveBeenCalledWith(
    "http://local/api/admin/viewers/a%2Fb/adjust",
    expect.objectContaining({ body: JSON.stringify(r) }),
  );
});
it("rejects malformed nested ledger without exposing partial data", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(
      new Response(
        JSON.stringify({ viewer_id: "v", gifts: [], ledger: [null] }),
      ),
    );
  await expect(
    createCompanionshipClient({ fetcher }).detail("v"),
  ).rejects.toThrow();
});
it("rejects malformed failed receipt diagnostics", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(
      new Response(
        JSON.stringify({
          durable_receipts: true,
          failed_receipts: [null],
          pending_receipts: 1,
          failed_receipt_attempts: 1,
        }),
      ),
    );
  await expect(
    createCompanionshipClient({ fetcher }).status(),
  ).rejects.toThrow();
});
