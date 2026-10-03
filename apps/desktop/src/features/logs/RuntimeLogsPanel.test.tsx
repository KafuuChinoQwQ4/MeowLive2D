import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { RuntimeLogsPanel } from "./RuntimeLogsPanel";
import { createRuntimeLogClient, type RuntimeLogClient, type RuntimeLogEntry } from "../../services/server/logs";

const entry = (id: string, summary: string, level: "info" | "error" = "info"): RuntimeLogEntry => ({ id, summary, level, timestamp: `2026-10-03T00:00:0${id}Z`, source: "server", category: "runtime", code: "started" });
const client = (entries: RuntimeLogEntry[]): RuntimeLogClient => ({ baseUrl: "http://test", report: vi.fn(), list: vi.fn().mockImplementation(async (query = {}) => ({ entries: entries.filter(item => !query.level || item.level === query.level), storage_available: true, truncated: false })) });
afterEach(() => vi.useRealTimers());
it("shows the newest 1000 records from merged sources", async () => {
  const records = Array.from({ length: 1001 }, (_, index) => ({ ...entry(String(index), `记录 ${index}`), timestamp: new Date(Date.UTC(2026, 9, 3) + index * 1000).toISOString() }));
  render(<RuntimeLogsPanel client={client(records.slice(0, 600))} launcherClient={client(records.slice(600).map(value => ({ ...value, source: "launcher" })))} />);
  await waitFor(() => expect(screen.getAllByRole("article")).toHaveLength(1000));
  expect(screen.queryByText("记录 0")).not.toBeInTheDocument();
  expect(screen.getAllByRole("article")[0]).toHaveTextContent("记录 1000");
});
it("sorts newest first and filters even cached results", async () => {
  render(<RuntimeLogsPanel client={client([entry("1", "旧记录"), entry("2", "新错误", "error")])} />);
  await screen.findByText("旧记录");
  expect(screen.getAllByRole("article").map(row => row.textContent)).toEqual([expect.stringContaining("新错误"), expect.stringContaining("旧记录")]);
  fireEvent.change(screen.getByLabelText("级别"), { target: { value: "error" } });
  await waitFor(() => expect(screen.queryByText("旧记录")).not.toBeInTheDocument());
  expect(screen.getByText("新错误")).toBeVisible();
});
it("can pause polling and refresh manually", async () => {
  vi.useFakeTimers();
  const logs = client([]);
  render(<RuntimeLogsPanel client={logs} pollIntervalMs={100} />);
  await act(async () => {});
  expect(logs.list).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "暂停自动刷新" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(500); });
  expect(logs.list).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "手动刷新" }));
  await act(async () => {});
  expect(logs.list).toHaveBeenCalledTimes(2);
});
it("keeps failed uploads observable while main service is offline", async () => {
  const logs = createRuntimeLogClient({ fetcher: vi.fn().mockRejectedValue(new Error("Bearer secret user prompt")) });
  await logs.report({ code: "browser_error" });
  render(<RuntimeLogsPanel client={logs} />);
  expect(await screen.findByText("浏览器运行时错误")).toBeVisible();
  expect(screen.getByText(/主服务日志暂不可用/)).toBeVisible();
  expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
});
it("shows launcher logs while the main service is unavailable", async () => {
  const logs = client([]); logs.list = vi.fn().mockRejectedValue(new Error("offline"));
  const launcher = client([{ ...entry("3", "启动器已就绪"), source: "launcher" }]);
  render(<RuntimeLogsPanel client={logs} launcherClient={launcher} />);
  expect(await screen.findByText("启动器已就绪")).toBeVisible();
  expect(screen.getByText(/主服务日志暂不可用/)).toBeVisible();
});
it("offers admin login without hiding the launcher logs", async () => {
  const { ServerRequestError } = await import("../../services/server/responses");
  const logs = client([]); logs.list = vi.fn().mockRejectedValue(new ServerRequestError("unauthorized", "", 401));
  const launcher = client([{ ...entry("3", "启动器已就绪"), source: "launcher" }]);
  render(<RuntimeLogsPanel client={logs} launcherClient={launcher} adminClient={{ baseUrl: "http://test", status: async () => ({ enabled: true, authenticated: false }), login: vi.fn(), logout: vi.fn(), subscribe: () => () => {} }} />);
  expect(await screen.findByRole("button", { name: "登录管理员" })).toBeVisible();
  expect(screen.getByText("启动器已就绪")).toBeVisible();
});
it("cancels obsolete filters and stops polling while hidden", async () => {
  const logs = client([]);
  let finish: ((value: { entries: RuntimeLogEntry[]; storage_available: boolean; truncated: boolean }) => void) | undefined;
  logs.list = vi.fn().mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }))
    .mockResolvedValue({ entries: [entry("2", "当前错误", "error")], storage_available: true, truncated: false });
  const view = render(<div><RuntimeLogsPanel client={logs} /></div>);
  await waitFor(() => expect(logs.list).toHaveBeenCalledTimes(1));
  const signal = vi.mocked(logs.list).mock.calls[0][1];
  fireEvent.change(screen.getByLabelText("级别"), { target: { value: "error" } });
  expect(await screen.findByText("当前错误")).toBeVisible();
  expect(signal?.aborted).toBe(true);
  await act(async () => { finish?.({ entries: [entry("1", "过期记录")], storage_available: true, truncated: false }); });
  expect(screen.queryByText("过期记录")).not.toBeInTheDocument();
  view.rerender(<div hidden><RuntimeLogsPanel client={logs} /></div>);
  await waitFor(() => expect(vi.mocked(logs.list).mock.calls[1][1]?.aborted).toBe(true));
});
it("shows native desktop logs immediately when HTTP is unavailable and preserves timestamps on updates", async () => {
  const logs = client([]); logs.list = vi.fn().mockRejectedValue(new Error("offline"));
  const native = { entries: [{ ...entry("3", "桌面执行端已停止", "error"), source: "desktop" }], storage_available: false, truncated: false };
  const view = render(<RuntimeLogsPanel client={logs} desktopLogs={native} />);
  expect(screen.getByText("桌面执行端已停止")).toBeVisible();
  expect(await screen.findByText(/主服务日志暂不可用/)).toBeVisible();
  view.rerender(<RuntimeLogsPanel client={logs} desktopLogs={{ ...native, entries: [...native.entries] }} />);
  fireEvent.change(screen.getByLabelText("来源"), { target: { value: "desktop" } });
  expect(screen.getAllByRole("article")).toHaveLength(1);
  expect(screen.getByText("桌面执行端已停止").closest("article")?.querySelector("time")).toHaveAttribute("datetime", native.entries[0].timestamp);
});
