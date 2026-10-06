import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SpeechPanel } from "./SpeechPanel";
import { createServerClient } from "../../services/server";
import { jsonResponse, serverStatus } from "../../test/server-fixtures";

describe("并行合成设置", () => {
  it("加载服务端设置，允许保存 16 并在重新进入后恢复", async () => {
    let saved = 4;
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url, init) => {
      if (String(url).endsWith("/api/speech/settings")) {
        if (init?.method === "POST") saved = JSON.parse(String(init.body)).sentence_batch_size;
        return jsonResponse({ sentence_batch_size: saved });
      }
      return jsonResponse(serverStatus());
    });
    const client = createServerClient({ fetcher });
    const user = userEvent.setup();
    const panel = render(<SpeechPanel client={client} />);
    const select = await screen.findByLabelText("并行合成分句数");
    await waitFor(() => expect(select).toHaveValue("4"));
    expect(screen.getAllByRole("option")).toHaveLength(16);
    await user.selectOptions(select, "16");
    await user.click(screen.getByRole("button", { name: "保存合成设置" }));
    await waitFor(() => expect(saved).toBe(16));
    panel.unmount();
    render(<SpeechPanel client={client} />);
    await waitFor(() => expect(screen.getByLabelText("并行合成分句数")).toHaveValue("16"));
  });
  it("保存失败时显示原因，并保留用户未保存的选择", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation(async (url, init) => {
      if (String(url).endsWith("/api/speech/settings")) return init?.method === "POST"
        ? jsonResponse({ code: "save_failed", message: "配置文件不可写" }, 500)
        : jsonResponse({ sentence_batch_size: 2 });
      return jsonResponse(serverStatus());
    });
    const user = userEvent.setup();
    render(<SpeechPanel client={createServerClient({ fetcher })} />);
    await waitFor(() => expect(screen.getByLabelText("并行合成分句数")).toHaveValue("2"));
    await user.selectOptions(screen.getByLabelText("并行合成分句数"), "8");
    await user.click(screen.getByRole("button", { name: "保存合成设置" }));
    expect(await screen.findByText("配置文件不可写")).toBeVisible();
    expect(screen.getByLabelText("并行合成分句数")).toHaveValue("8");
  });
});
