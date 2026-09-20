import { act, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import type { RuntimePresetSnapshot, TrainingJob, TrainingSnapshot } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { ServerRequestError } from "../../services/server/responses";
import { pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";
import { useTraining } from "./useTraining";
import { FeedbackProvider } from "../../app/feedback/OperationFeedback";
const job: TrainingJob = { id: "job-1", name: "我的音色", voice_id: "voice-1", status: "training", progress: 40, message: "训练中", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: null, performance: { batch_size: 1, data_workers: 1, cpu_threads: 2, gpu_index: 0, low_memory: true } };
const preset: RuntimePresetSnapshot = { mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" };
function setup(jobs: TrainingJob[] = []) {
  let snapshot: TrainingSnapshot = { enabled: true, busy: false, jobs, versions: [] };
  const client: TrainingClient = {
    snapshot: vi.fn(async () => snapshot), modelStatus: vi.fn().mockResolvedValue({ supported: true, state: "loaded", message: "模型已启用" }), setModelsEnabled: vi.fn(),
    create: vi.fn().mockResolvedValue(job), transcribe: vi.fn(), cancel: vi.fn(), delete: vi.fn(), save: vi.fn(), activate: vi.fn(), audition: vi.fn(),
    preset: vi.fn().mockResolvedValue(preset), measure: vi.fn(),
  };
  const resources = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources, setJobs: (jobs: TrainingJob[]) => { snapshot = { ...snapshot, jobs }; } };
}
async function openMeasurement() {
  const user = userEvent.setup();
  await screen.findByText("训练就绪");
  await user.click(screen.getByRole("tab", { name: "离线设置" }));
  await user.selectOptions(screen.getByLabelText("测量参考音色"), "voice-1");
  await user.click(screen.getByRole("button", { name: "测量本地 LLM 与 TTS" }));
  return user;
}
it("GPU 失败以弹窗展示原始原因和改正建议，关闭后不被轮询重复弹出", async () => {
  const deps = setup();
  vi.mocked(deps.client.measure).mockRejectedValue(new ServerRequestError("gpu_unavailable", "GPU 采样失败", 409));
  render(<TrainingPanel {...deps} />);
  const user = await openMeasurement();
  const dialog = await screen.findByRole("dialog", { name: "离线测量失败" });
  expect(within(dialog).getByText("GPU 采样失败")).toBeVisible();
  expect(within(dialog).getByText(/nvidia-smi/)).toBeVisible();
  expect(within(dialog).getByText(/GPU 编号/)).toBeVisible();
  await user.click(within(dialog).getByRole("button", { name: "知道了" }));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "测量本地 LLM 与 TTS" })).toHaveFocus();
});
it.each([true, false])("HTTP 成功也根据 verified=%s 判断测量通过与否", async verified => {
  const deps = setup();
  vi.mocked(deps.client.measure).mockResolvedValue({ ...preset, verified, message: verified ? "本次测量通过" : "实测未通过：显存峰值超过限制" });
  render(<TrainingPanel {...deps} />);
  await openMeasurement();
  const dialog = await screen.findByRole(verified ? "status" : "dialog", { name: verified ? "离线测量成功" : "离线测量失败" });
  expect(dialog).toHaveTextContent(verified ? /本次测量通过/ : /显存峰值超过限制/);
});
it("提交只提示任务已接收，不把排队当作训练完成", async () => {
  const deps = setup(); render(<TrainingPanel {...deps} />);
  const user = userEvent.setup(); await screen.findByText("训练就绪");
  await user.type(screen.getByLabelText("训练名称"), "我的训练");
  await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
  await user.upload(screen.getByLabelText("训练片段"), [pcm16Wav(), pcm16Wav()]);
  await user.click(screen.getByRole("button", { name: "开始训练" }));
  const dialog = await screen.findByRole("status", { name: "训练已提交" });
  expect(dialog).toHaveTextContent("训练记录");
  expect(screen.queryByRole("dialog", { name: "训练完成" })).not.toBeInTheDocument();
});
it.each(["completed", "failed"])("后台任务转为 %s 时提示一次，历史终态与重复轮询不弹", async status => {
  vi.useFakeTimers(); const deps = setup([{ ...job, id: "old", status: "failed" }, job]);
  render(<TrainingPanel {...deps} />);
  await act(async () => {});
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  deps.setJobs([{ ...job, id: "old", status: "failed" }, { ...job, status, message: status === "completed" ? "权重已保存" : "CUDA 显存不足" }]);
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  const dialog = screen.getByRole(status === "completed" ? "status" : "dialog", { name: status === "completed" ? "训练完成" : "训练失败" });
  expect(dialog).toHaveTextContent("我的音色");
  if (status === "failed") {
    expect(dialog).toHaveTextContent("降低");
    fireEvent.click(within(dialog).getByRole("button", { name: "知道了" }));
  }
  await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});
it("旧服务的迟到失败不在新服务弹窗", async () => {
  const deps = setup(); let reject!: (error: Error) => void;
  vi.mocked(deps.client.measure).mockImplementation(() => new Promise((_, fail) => { reject = fail; }));
  const view = render(<TrainingPanel {...deps} />); await openMeasurement();
  view.rerender(<TrainingPanel {...setup()} />);
  await act(async () => { reject(new Error("旧服务失败")); });
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
});
it("历史失败记录可主动查看原因和建议，不需要重跑任务", async () => {
  const deps = setup([{ ...job, status: "failed", message: "GPU 采样失败" }]);
  render(<TrainingPanel {...deps} />); const user = userEvent.setup();
  await screen.findByText("训练就绪");
  await user.click(screen.getByRole("tab", { name: /训练记录/ }));
  await user.click(screen.getByText("查看任务详情"));
  await user.click(screen.getByRole("button", { name: "查看结果与建议" }));
  expect(await screen.findByRole("dialog", { name: "训练失败" })).toHaveTextContent("nvidia-smi");
});
it("请求等待时按钮失焦，关闭结果弹窗仍回到发起操作的按钮", async () => {
  const deps = setup(); let reject!: (error: Error) => void;
  vi.mocked(deps.client.measure).mockImplementation(() => new Promise((_, fail) => { reject = fail; }));
  render(<TrainingPanel {...deps} />); const user = await openMeasurement();
  document.body.tabIndex = -1; document.body.focus();
  await act(async () => { reject(new Error("GPU 采样失败")); });
  await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "知道了" }));
  expect(screen.getByRole("button", { name: "测量本地 LLM 与 TTS" })).toHaveFocus();
  document.body.removeAttribute("tabindex");
});
it("提交后的旧轮询不会清掉新任务追踪，最终失败仍弹窗", async () => {
  vi.useFakeTimers();
  const deps = setup();
  const empty: TrainingSnapshot = { enabled: true, busy: false, jobs: [], versions: [] };
  let resolveCreate!: (job: TrainingJob) => void;
  let resolveOldPoll!: (value: TrainingSnapshot) => void;
  const created = new Promise<TrainingJob>(resolve => { resolveCreate = resolve; });
  const oldPoll = new Promise<TrainingSnapshot>(resolve => { resolveOldPoll = resolve; });
  vi.mocked(deps.client.snapshot).mockResolvedValueOnce(empty).mockReturnValueOnce(oldPoll)
    .mockResolvedValue({ ...empty, jobs: [{ ...job, status: "failed", message: "训练进程失败" }] });
  const { result } = renderHook(() => useTraining(deps.client, deps.resources));
  await act(async () => {});
  let action!: Promise<void>;
  act(() => {
    action = result.current.action(async () => {
      result.current.trackJob(await created);
    }, { operation: "提交训练", success: "任务已接收" });
  });
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(deps.client.snapshot).toHaveBeenCalledTimes(2);
  await act(async () => {
    resolveCreate(job);
    resolveOldPoll(empty);
    await action;
  });
  expect(result.current.snapshot?.jobs[0]?.status).toBe("failed");
  expect(result.current.notice?.title).toBe("训练失败");
  act(() => result.current.dismissNotice());
  expect(result.current.notice?.kind).toBe("success");
});
it("控制面板共享弹窗展示训练错误且不出现第二个本地弹窗", async () => {
  const deps = setup();
  vi.mocked(deps.client.measure).mockRejectedValue(new Error("GPU 采样失败"));
  render(<FeedbackProvider><TrainingPanel {...deps} /></FeedbackProvider>);
  await openMeasurement();
  const dialog = await screen.findByRole("dialog", { name: "离线测量失败" });
  expect(dialog).toHaveClass("operation-result-dialog");
  expect(screen.getAllByRole("dialog")).toHaveLength(1);
  expect(dialog).toHaveTextContent("nvidia-smi");
});
it("尚未启动 TTS 时只显示开关说明，已运行的模型中断才弹窗", async () => {
  vi.useFakeTimers();
  const deps = setup();
  const unavailable = { supported: true, state: "unavailable", message: "TTS 服务未就绪，请先启动 TTS" };
  vi.mocked(deps.client.modelStatus).mockResolvedValue(unavailable);
  render(<FeedbackProvider><TrainingPanel {...deps} /></FeedbackProvider>);
  await act(async () => {});
  expect(screen.getByText(unavailable.message)).toBeVisible();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  vi.mocked(deps.client.modelStatus).mockResolvedValue({ supported: true, state: "loaded", message: "模型已启用" });
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  vi.mocked(deps.client.modelStatus).mockResolvedValue(unavailable);
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(screen.getByRole("dialog")).toHaveTextContent(unavailable.message);
});
it("状态读取失败弹窗只提示一次，恢复后再次失败可重新提醒", async () => {
  vi.useFakeTimers();
  const deps = setup();
  vi.mocked(deps.client.snapshot).mockRejectedValue(new Error("训练服务连接中断"));
  render(<FeedbackProvider><TrainingPanel {...deps} /></FeedbackProvider>);
  await act(async () => {});
  fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "知道了" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(4000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  vi.mocked(deps.client.snapshot).mockResolvedValue({ enabled: true, busy: false, jobs: [], versions: [] });
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  vi.mocked(deps.client.snapshot).mockRejectedValue(new Error("训练服务再次中断"));
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(screen.getByRole("dialog")).toHaveTextContent("训练服务再次中断");
});
