import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";
import { resourceSnapshot } from "../../test/resource-fixtures";
import { ResourcesPanel } from "./ResourcesPanel";

describe("资源面板生命周期", () => {
  it("训练页选用音色后刷新同页的当前音色标记", async () => {
    let snapshot = resourceSnapshot({ active_voice_id: "" });
    const resourceClient = { baseUrl: "http://127.0.0.1:19600", getSnapshot: vi.fn(async () => snapshot) } as unknown as ResourcesClient;
    const view = render(<ResourcesPanel mode="voices" resourceClient={resourceClient} speechClient={{} as ServerClient} refreshToken={0} />);
    await screen.findByRole("heading", { name: "音色管理" });
    snapshot = resourceSnapshot({ active_voice_id: "voice-1" });
    view.rerender(<ResourcesPanel mode="voices" resourceClient={resourceClient} speechClient={{} as ServerClient} refreshToken={1} />);
    expect(await screen.findByText("当前音色")).toBeVisible();
  });

  it("卸载时取消仍在等待的资源快照请求", async () => {
    let signal: AbortSignal | undefined;
    const resourceClient = {
      baseUrl: "http://127.0.0.1:19600",
      getSnapshot: vi.fn((_signal?: AbortSignal) => {
        signal = _signal;
        return new Promise(() => undefined);
      }),
    } as unknown as ResourcesClient;
    const speechClient = {} as ServerClient;
    const { ResourcesPanel } = await import("./ResourcesPanel");
    const view = render(<ResourcesPanel resourceClient={resourceClient} speechClient={speechClient} />);

    view.unmount();

    expect(signal?.aborted).toBe(true);
  });

  it("卸载时也取消用户发起的桌面资源操作", async () => {
    const user = userEvent.setup();
    let actionSignal: AbortSignal | undefined;
    const resourceClient = {
      baseUrl: "http://127.0.0.1:19600",
      getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()),
      desktop: vi.fn((_operation, signal?: AbortSignal) => {
        actionSignal = signal;
        return new Promise(() => undefined);
      }),
    } as unknown as ResourcesClient;
    const { ResourcesPanel } = await import("./ResourcesPanel");
    const view = render(<ResourcesPanel resourceClient={resourceClient} speechClient={{} as ServerClient} />);
    await screen.findByRole("heading", { name: "角色管理" });
    await user.click(screen.getByRole("button", { name: "刷新 VTS 模型" }));

    view.unmount();

    expect(actionSignal?.aborted).toBe(true);
  });
});
