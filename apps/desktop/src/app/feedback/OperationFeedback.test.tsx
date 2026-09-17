import { act, fireEvent, render, renderHook, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { FeedbackProvider, useFeedback } from "./OperationFeedback";
import { useLauncher } from "../../features/launcher/useLauncher";
import { launcherSnapshot } from "../../test/launcher-fixtures";
import type { LauncherClient } from "../../services/launcher";

it("queues operation results and restores the original focused control after dismissal", () => {
  function Actions() {
    const feedback = useFeedback();
    return <button onClick={() => {
      feedback.success("保存成功", "配置已保存");
      feedback.error("连接失败", new Error("服务离线"));
    }}>操作</button>;
  }
  render(<FeedbackProvider><Actions /></FeedbackProvider>);
  const trigger = screen.getByRole("button", { name: "操作" });
  trigger.focus(); fireEvent.click(trigger);
  expect(screen.getByRole("dialog", { name: "保存成功" })).toHaveTextContent("配置已保存");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  expect(screen.getByRole("dialog", { name: "连接失败" })).toHaveTextContent("服务离线");
  fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
  fireEvent(screen.getByRole("dialog"), new Event("cancel", { bubbles: false, cancelable: true }));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
});

it("reports one passive failure per episode but reports an explicit repeated action failure", () => {
  const { result } = renderHook(useFeedback, { wrapper: FeedbackProvider });
  act(() => result.current.reportIssue("status", "状态读取失败", "离线"));
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  act(() => result.current.reportIssue("status", "状态读取失败", "仍然离线"));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  act(() => result.current.error("重试失败", "仍然离线"));
  expect(screen.getByRole("dialog", { name: "重试失败" })).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  act(() => { result.current.clearIssue("status"); result.current.reportIssue("status", "状态读取失败", "再次离线"); });
  expect(screen.getByRole("dialog")).toHaveTextContent("再次离线");
});

it("keeps focus restoration when another result arrives while the first dialog is open", () => {
  let notify!: ReturnType<typeof useFeedback>["notify"];
  function Actions() { notify = useFeedback().notify; return <button>开始操作</button>; }
  render(<FeedbackProvider><Actions /></FeedbackProvider>);
  const trigger = screen.getByRole("button", { name: "开始操作" });
  trigger.focus();
  act(() => notify({ kind: "info", title: "已提交", message: "处理中" }));
  act(() => notify({ kind: "error", title: "处理失败", message: "设备断开" }));
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  expect(trigger).toHaveFocus();
});

it("acknowledges a launcher start as accepted and shows a later service failure once", async () => {
  vi.useFakeTimers();
  let snapshot = launcherSnapshot();
  const client: LauncherClient = { getStatus: vi.fn(async () => snapshot), setEnabled: vi.fn(async () => launcherSnapshot("starting")) };
  const { result } = renderHook(() => useLauncher(client), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.setEnabled("server", true); });
  expect(screen.getByRole("dialog")).toHaveTextContent("已提交");
  expect(screen.getByRole("dialog")).not.toHaveTextContent("启动成功");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  snapshot = launcherSnapshot("failed"); snapshot.services[0].message = "服务进程退出";
  await act(async () => { await vi.advanceTimersByTimeAsync(1500); });
  expect(screen.getByRole("dialog")).toHaveTextContent("服务进程退出");
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "知道了" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(3000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("ignores a launcher polling request that fails after the hook is unmounted", async () => {
  let reject!: (error: Error) => void;
  const client: LauncherClient = { getStatus: vi.fn(() => new Promise<ReturnType<typeof launcherSnapshot>>((_, fail) => { reject = fail; })), setEnabled: vi.fn() };
  function Poll() { useLauncher(client); return null; }
  const view = render(<FeedbackProvider><Poll /></FeedbackProvider>);
  view.rerender(<FeedbackProvider />);
  await act(async () => { reject(new Error("旧请求失败")); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("shows launcher readiness once when an accepted start completes", async () => {
  vi.useFakeTimers(); let snapshot = launcherSnapshot();
  const client: LauncherClient = { getStatus: vi.fn(async () => snapshot), setEnabled: vi.fn().mockResolvedValue(launcherSnapshot("starting")) };
  const { result } = renderHook(() => useLauncher(client), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.setEnabled("server", true); });
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  snapshot = launcherSnapshot("running");
  await act(async () => { await vi.advanceTimersByTimeAsync(1500); });
  expect(screen.getByRole("dialog")).toHaveTextContent("主服务已就绪");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(3000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});
