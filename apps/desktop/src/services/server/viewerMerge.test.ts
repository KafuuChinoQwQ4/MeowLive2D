import { expect, it, vi } from "vitest";
import { createViewerMergeClient } from "./viewerMerge";
it("preserves preview revision and fingerprint in confirmed apply", async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue(
      new Response(JSON.stringify({ canonical_viewer_id: "b", revision: 4 })),
    );
  const r = {
    source_viewer_id: "a",
    target_viewer_id: "b",
    expected_revision: 3,
    fingerprint: "proof",
    request_key: "retry",
    reason: "相同账号核实",
    confirmed: true,
  };
  await createViewerMergeClient({ baseUrl: "http://x", fetcher }).apply(r);
  expect(fetcher).toHaveBeenCalledWith(
    "http://x/api/admin/viewers/merge/apply",
    expect.objectContaining({ body: JSON.stringify(r) }),
  );
});
