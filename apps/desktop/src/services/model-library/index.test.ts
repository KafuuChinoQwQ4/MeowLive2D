import { describe, expect, it, vi } from "vitest";
import { modelLibrarySnapshot } from "../../test/model-library-fixtures";
import { jsonResponse } from "../../test/server-fixtures";
import { createModelLibraryClient } from ".";

describe("model library service", () => {
  it("sends only the curated model ID with the launcher session token", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(modelLibrarySnapshot()));
    await createModelLibraryClient({ fetcher }).download("gpt-sovits-v2", "a".repeat(64));
    expect(fetcher).toHaveBeenCalledWith("/api/launcher/models/download", expect.objectContaining({ method: "POST", body: '{"id":"gpt-sovits-v2"}', headers: { "Content-Type": "application/json", "X-MeowLive-Launcher-Token": "a".repeat(64) } }));
  });
  it("rejects malformed environment, paths, and unsafe official links", async () => {
    for (const snapshot of [modelLibrarySnapshot({ schema_version: 2 }), modelLibrarySnapshot({ installed: [{ ...modelLibrarySnapshot().installed[0], path: 42 } as never] }), modelLibrarySnapshot({ catalog: [{ ...modelLibrarySnapshot().catalog[0], source_url: "javascript:alert(1)" }] })]) {
      await expect(createModelLibraryClient({ fetcher: vi.fn().mockResolvedValue(jsonResponse(snapshot)) }).getStatus()).rejects.toThrow(/无效/);
    }
  });
  it("reports download failures without replaying mutations", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ message: "磁盘空间不足" }, 409));
    await expect(createModelLibraryClient({ fetcher }).download("model", "a".repeat(64))).rejects.toThrow("磁盘空间不足");
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
  it("times out even when fetch ignores abort", async () => {
    await expect(createModelLibraryClient({ fetcher: () => new Promise(() => {}), timeoutMs: 15 }).getStatus()).rejects.toThrow(/超时/);
  });
});
