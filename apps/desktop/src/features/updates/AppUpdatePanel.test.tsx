import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppUpdatePanel } from "./AppUpdatePanel";
import type { UpdateClient } from "../../services/updates";
const status = { phase: "available", current_tag: "v0.1.1-windows-preview.20260929", available_tag: "v0.1.1-windows-preview.20261004", release_url: "https://github.com/KafuuChinoQwQ4/MeowLive2D/releases/tag/v0.1.1-windows-preview.20261004", release_notes: "修复问题", message: "发现可用更新", downloaded_bytes: 0, reused_bytes: 0, total_bytes: 100 };
function client(): UpdateClient { return { status: vi.fn().mockResolvedValue(status), check: vi.fn().mockResolvedValue(undefined), prepare: vi.fn().mockResolvedValue(undefined), cancel: vi.fn().mockResolvedValue(undefined), install: vi.fn().mockResolvedValue(undefined) }; }
describe("App updates", () => {
  it("checks on entry, shows release notes and starts download", async () => {
    const api = client(); render(<AppUpdatePanel client={api} />);
    await screen.findByText("发现可用更新");
    expect(api.check).toHaveBeenCalledTimes(1);
    expect(screen.getByText("修复问题")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "下载更新" }));
    await waitFor(() => expect(api.prepare).toHaveBeenCalledTimes(1));
  });
  it("disables update actions during an environment operation", async () => {
    const api = client(); render(<AppUpdatePanel client={api} disabled />);
    await screen.findByText("发现可用更新");
    expect(screen.getByRole("button", { name: "下载更新" })).toBeDisabled();
  });
  it("reports real reused bytes and requires an explicit install click", async () => {
    const api = client(); vi.mocked(api.status).mockResolvedValue({ ...status, phase: "ready", reused_bytes: 50, downloaded_bytes: 50 });
    render(<AppUpdatePanel client={api} />);
    await screen.findByText(/已复用/);
    expect(api.install).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "安装并重启" }));
    await waitFor(() => expect(api.install).toHaveBeenCalledTimes(1));
  });
});
