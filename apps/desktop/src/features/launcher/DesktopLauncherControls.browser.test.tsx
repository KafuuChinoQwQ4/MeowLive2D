import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { openControlPanel } from "../../services/desktop";
import { DesktopLauncherControls } from "./DesktopLauncherControls";

vi.mock("../../services/desktop", async () => {
  const actual = await vi.importActual<typeof import("../../services/desktop")>("../../services/desktop");
  return { ...actual, openControlPanel: vi.fn().mockResolvedValue(undefined) };
});

const status = {
  config_path: "desktop.toml", server_url: "http://127.0.0.1:19600",
  server: { ready: true, managed: true, last_error: null, log_path: "server.log" },
  runtime: { running: true, simulation: false, last_error: null },
};

it("shows the browser control panel address and opens it through the desktop boundary", async () => {
  render(<DesktopLauncherControls status={status} busy={false} stale={false} setEnabled={vi.fn()} refresh={vi.fn()} />);
  expect(screen.getByText("http://127.0.0.1:1420")).toBeVisible();
  expect(screen.getByText(/保持此 App 运行即可在浏览器访问/)).toBeVisible();
  await userEvent.click(screen.getByRole("button", { name: "在浏览器中打开" }));
  expect(openControlPanel).toHaveBeenCalledOnce();
});

it("keeps all service controls visible when the optional browser listener fails", () => {
  render(<DesktopLauncherControls status={{ ...status, browser_panel_error: "无法监听 127.0.0.1:1420，端口已被占用" }} busy={false} stale={false} setEnabled={vi.fn()} refresh={vi.fn()} />);
  expect(screen.getByRole("alert")).toHaveTextContent("端口已被占用");
  expect(screen.getAllByRole("switch")).toHaveLength(3);
  expect(screen.getByRole("switch", { name: "主服务" })).toBeChecked();
  expect(screen.getByRole("button", { name: "在浏览器中打开" })).toBeDisabled();
});

it("shows the same three cards and browser entry on Linux with unavailable controls disabled", () => {
  render(<DesktopLauncherControls status={{ ...status, platform: "linux", runtime: { running: false, simulation: false, last_error: null } }} busy={false} stale={false} setEnabled={vi.fn()} refresh={vi.fn()} />);
  expect(screen.getAllByRole("switch")).toHaveLength(3);
  expect(screen.getByRole("switch", { name: "Windows 执行端" })).toBeDisabled();
  expect(screen.getByText(/保持此 App 运行即可在浏览器访问/)).toBeVisible();
});
