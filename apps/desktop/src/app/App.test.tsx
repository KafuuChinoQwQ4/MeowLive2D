import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { agentStatus } from "../test/agent-fixtures";
import { liveSnapshot } from "../test/live-fixtures";
import { resourceSnapshot } from "../test/resource-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
  window.history.replaceState(null, "", "/");
});

describe("桌面控制台", () => {
  it("按导航展示单个功能，并在切页后保留播报与 Agent 输入", async () => {
    vi.stubGlobal("fetch", vi.fn<typeof fetch>().mockImplementation(async (url) => {
      if (String(url).endsWith("/api/live")) return jsonResponse(liveSnapshot());
      if (String(url).endsWith("/api/resources")) return jsonResponse(resourceSnapshot());
      return String(url).endsWith("/api/agent") ? jsonResponse(agentStatus()) : jsonResponse(serverStatus());
    }));
    const { App } = await import("./App");

    render(<App />);

    const speechLink = await screen.findByRole("link", { name: "语音播报" });
    await userEvent.click(speechLink);
    const input = await screen.findByRole("textbox", { name: "播报文本" });
    await userEvent.type(input, "保留这句话");
    await userEvent.click(screen.getByRole("link", { name: "Agent 互动" }));
    const persona = await screen.findByRole("textbox", { name: "主播人设" });
    await userEvent.clear(persona); await userEvent.type(persona, "保留这个人设");
    expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument();
    await userEvent.click(speechLink);
    expect(await screen.findByRole("textbox", { name: "播报文本" })).toHaveValue("保留这句话");
    expect(screen.queryByRole("heading", { name: "Agent 已暂停" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("link", { name: "Agent 互动" }));
    expect(await screen.findByRole("textbox", { name: "主播人设" })).toHaveValue("保留这个人设");
    expect(screen.getByRole("link", { name: "Agent 互动" })).toHaveAttribute("aria-current", "page");
    await waitFor(() => expect(window.location.hash).toBe("#agent"));
  });
});
