import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { launcherSnapshot } from "../../test/launcher-fixtures";
import type { LauncherClient } from "../../services/launcher";
import { LauncherPanel } from "./LauncherPanel";

function client(status = launcherSnapshot()): LauncherClient {
  return { getStatus: vi.fn().mockResolvedValue(status), setEnabled: vi.fn().mockResolvedValue(status) };
}

it("shows automatic startup and lets the user stop the main service from its switch", async () => {
  const api = client(launcherSnapshot("running"));
  vi.mocked(api.setEnabled).mockResolvedValueOnce(launcherSnapshot("stopped"));
  render(<LauncherPanel client={api}><h2>文字播报</h2></LauncherPanel>);
  await screen.findByText(/主服务会自动启动/);
  const toggle = screen.getByRole("switch", { name: "主服务" });
  expect(toggle).toBeChecked();
  await userEvent.click(toggle);
  expect(api.setEnabled).toHaveBeenLastCalledWith("server", false, "a".repeat(64), expect.any(AbortSignal));
  await waitFor(() => expect(toggle).not.toBeChecked());
  expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument();
  expect(screen.getByRole("switch", { name: "Windows 执行端" })).toBeDisabled();
});

it("links LLM setup to the configuration page and explains that launcher status updates after restart", async () => {
  render(<LauncherPanel client={client()} />);
  expect(await screen.findByRole("link", { name: "前往 LLM 接入" })).toHaveAttribute("href", "#llm");
  expect(screen.getByText(/保存后重启主服务/)).toBeVisible();
});

it("shows automatic startup progress until the server is ready", async () => {
  render(<LauncherPanel client={client(launcherSnapshot("starting"))}><h2>文字播报</h2></LauncherPanel>);
  expect(await screen.findByText("启动中")).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument();
  const toggle = screen.getByRole("switch", { name: "主服务" });
  expect(toggle).toBeChecked();
  expect(toggle).toBeEnabled();
});

it("shows the business controls for a ready server and explains how to stop an external TTS", async () => {
  render(<LauncherPanel client={client(launcherSnapshot("running", "external"))}><h2>文字播报</h2></LauncherPanel>);
  expect(await screen.findByRole("heading", { name: "文字播报" })).toBeInTheDocument();
  expect(screen.getByRole("switch", { name: "TTS 语音引擎" })).toBeDisabled();
  expect(screen.getByText(/其他终端/)).toBeInTheDocument();
});

it("a refused start leaves the switch off and displays the failure", async () => {
  const api = client();
  vi.mocked(api.setEnabled).mockRejectedValue(new Error("端口被占用"));
  render(<LauncherPanel client={api} />);
  const toggle = await screen.findByRole("switch", { name: "TTS 语音引擎" });
  await waitFor(() => expect(toggle).toBeEnabled());
  await userEvent.click(toggle);
  expect(await screen.findByRole("alert")).toHaveTextContent("端口被占用");
  expect(toggle).not.toBeChecked();
});

it("an unavailable manager disables switches and explains the single startup command", async () => {
  const api = client();
  vi.mocked(api.getStatus).mockRejectedValue(new Error("无法连接启动管理"));
  render(<LauncherPanel client={api} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("./launchers/start.sh");
  expect(screen.getByRole("switch", { name: "TTS 语音引擎" })).toBeDisabled();
});


it("keeps Windows disconnected until the main service is ready and shows the backend reason", async () => {
  const status = launcherSnapshot("starting");
  status.services[2].message = "请先等待主服务就绪，再连接 Windows 执行端。";
  render(<LauncherPanel client={client(status)} />);
  await screen.findByText("请先等待主服务就绪，再连接 Windows 执行端。");
  const toggle = screen.getByRole("switch", { name: "Windows 执行端" });
  expect(toggle).toBeDisabled();
  expect(toggle).not.toBeChecked();
  expect(screen.getByText("未连接")).toBeInTheDocument();
});

it("connects Windows through the switch and can cancel while connecting", async () => {
  const api = client(launcherSnapshot("running"));
  vi.mocked(api.setEnabled).mockResolvedValueOnce(launcherSnapshot("running", "stopped", "starting"))
    .mockResolvedValueOnce(launcherSnapshot("running", "stopped", "stopping"));
  render(<LauncherPanel client={api} />);
  const toggle = await screen.findByRole("switch", { name: "Windows 执行端" });
  await waitFor(() => expect(toggle).toBeEnabled());
  await userEvent.click(toggle);
  expect(api.setEnabled).toHaveBeenLastCalledWith("windows", true, "a".repeat(64), expect.any(AbortSignal));
  expect(await screen.findByText("连接中")).toBeInTheDocument();
  expect(toggle).toBeChecked();
  await userEvent.click(toggle);
  expect(api.setEnabled).toHaveBeenLastCalledWith("windows", false, "a".repeat(64), expect.any(AbortSignal));
  await waitFor(() => expect(toggle).toBeDisabled());
});

it("shows a connected Windows client and disconnects only the managed client", async () => {
  const api = client(launcherSnapshot("running", "running", "running"));
  vi.mocked(api.setEnabled).mockResolvedValue(launcherSnapshot("running", "running", "stopped"));
  render(<LauncherPanel client={api} />);
  expect(await screen.findByText("已连接")).toBeInTheDocument();
  const toggle = screen.getByRole("switch", { name: "Windows 执行端" });
  expect(toggle).toBeChecked();
  await userEvent.click(toggle);
  expect(api.setEnabled).toHaveBeenLastCalledWith("windows", false, "a".repeat(64), expect.any(AbortSignal));
  await waitFor(() => expect(toggle).not.toBeChecked());
});

it("does not stop a Windows client connected from elsewhere", async () => {
  render(<LauncherPanel client={client(launcherSnapshot("running", "running", "external"))} />);
  const toggle = await screen.findByRole("switch", { name: "Windows 执行端" });
  await waitFor(() => expect(toggle).toBeChecked());
  expect(toggle).toBeDisabled();
  expect(screen.getByText(/其他执行端已连接/)).toBeInTheDocument();
});

it("shows Windows startup failures and allows retrying the connection", async () => {
  const status = launcherSnapshot("running", "running", "failed");
  status.services[2].message = "Windows 执行端未能连接，请检查日志后重试。";
  const api = client(status);
  vi.mocked(api.setEnabled).mockResolvedValue(launcherSnapshot("running", "running", "starting"));
  render(<LauncherPanel client={api} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Windows 执行端未能连接");
  const toggle = screen.getByRole("switch", { name: "Windows 执行端" });
  expect(toggle).not.toBeChecked();
  expect(toggle).toBeEnabled();
  await userEvent.click(toggle);
  expect(await screen.findByText("连接中")).toBeInTheDocument();
});
