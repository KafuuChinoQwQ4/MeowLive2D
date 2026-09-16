import { describe, expect, it, vi } from "vitest";
import { launcherSnapshot } from "../../test/launcher-fixtures";
import { jsonResponse } from "../../test/server-fixtures";
import { createLauncherClient } from ".";

describe("local launcher service", () => {
  it("sends the explicit switch state and session token, without command or config fields", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(launcherSnapshot("starting")));
    const client = createLauncherClient({ fetcher });
    await client.setEnabled("server", true, "a".repeat(64));
    expect(fetcher).toHaveBeenCalledWith("/api/launcher/services/server", expect.objectContaining({ method: "POST",
      headers: { "Content-Type": "application/json", "X-MeowLive-Launcher-Token": "a".repeat(64) }, body: '{"enabled":true}' }));
  });
  it("rejects HTML fallback and malformed launcher status", async () => {
    for (const response of [new Response("<html>Vite fallback</html>"), jsonResponse({ ...launcherSnapshot(), services: [] })]) {
      await expect(createLauncherClient({ fetcher: vi.fn().mockResolvedValue(response) }).getStatus()).rejects.toThrow(/启动管理/);
    }
  });
  it("reports an explicit start refusal without retrying the mutation", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ message: "端口被占用" }, 409));
    await expect(createLauncherClient({ fetcher }).setEnabled("tts", true, "a".repeat(64))).rejects.toThrow("端口被占用");
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
  it("bounds hung requests even when the fetch implementation ignores cancellation", async () => {
    const client = createLauncherClient({ fetcher: () => new Promise(() => {}), timeoutMs: 20 });
    await expect(client.getStatus()).rejects.toThrow(/超时/);
  });
});


it("accepts the Windows service and sends its explicit connection toggle", async () => {
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(launcherSnapshot("running", "running", "starting")));
  const client = createLauncherClient({ fetcher });
  const result = await client.setEnabled("windows", true, "a".repeat(64));
  expect(result.services.find(service => service.id === "windows")?.state).toBe("starting");
  expect(fetcher).toHaveBeenCalledWith("/api/launcher/services/windows", expect.objectContaining({ method: "POST",
    headers: { "Content-Type": "application/json", "X-MeowLive-Launcher-Token": "a".repeat(64) }, body: '{"enabled":true}' }));
});

it("rejects missing, duplicated, and unknown services in the three-service snapshot", async () => {
  const valid = launcherSnapshot();
  for (const services of [valid.services.slice(0, 2), [valid.services[0], valid.services[1], valid.services[1]],
    [...valid.services, valid.services[2]], [valid.services[0], valid.services[1], { ...valid.services[2], id: "shell" }]]) {
    await expect(createLauncherClient({ fetcher: vi.fn().mockResolvedValue(jsonResponse({ ...valid, services })) }).getStatus()).rejects.toThrow(/无效状态/);
  }
});
