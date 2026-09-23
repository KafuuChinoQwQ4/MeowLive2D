import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { TrainingSnapshot } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";

function setup(count = 0) {
  const performance = { batch_size: 1, data_workers: 1, cpu_threads: 2, gpu_index: 0, low_memory: true };
  const snapshot: TrainingSnapshot = { enabled: true, busy: false, jobs: Array.from({ length: count }, (_, i) => ({ id: `job-${i}`, name: `历史训练 ${i}`, voice_id: "voice-1", status: "completed", progress: 100, message: `完成详情 ${i}`, clip_count: 2, created_at_ms: i, updated_at_ms: i, version_id: `job-${i}`, performance })), versions: Array.from({ length: count }, (_, i) => ({ id: `job-${i}`, job_id: `job-${i}`, voice_id: "voice-1", name: `已训练音色 ${i}`, engine: "gpt-sovits", model_version: "v2", auditioned: true, saved: true, active: i === 0, available: true, created_at_ms: i })) };
  let loaded = false;
  const client: TrainingClient = {
    snapshot: vi.fn().mockResolvedValue(snapshot),
    modelStatus: vi.fn(async () => ({ supported: true, state: loaded ? "loaded" : "unloaded", message: loaded ? "模型已启用" : "模型已关闭" })),
    setModelsEnabled: vi.fn(async enabled => { loaded = enabled; return { supported: true, state: loaded ? "loaded" : "unloaded", message: loaded ? "模型已启用" : "模型已关闭" }; }),
    create: vi.fn().mockResolvedValue(snapshot.jobs[0]), transcribe: vi.fn(), cancel: vi.fn(), delete: vi.fn(), save: vi.fn(), activate: vi.fn().mockResolvedValue(snapshot), audition: vi.fn(), measure: vi.fn(),
    preset: vi.fn().mockResolvedValue({ mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" }),
  };
  const resources = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()), selectVoice: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources };
}

describe("训练工作区", () => {
  it("分页整理素材，切换页签保留草稿并提交自定义性能选项", async () => {
    const deps = setup(), user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    await user.type(screen.getByLabelText("训练名称"), "自定义训练");
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    await user.selectOptions(screen.getByLabelText("性能预设"), "custom");
    fireEvent.change(screen.getByLabelText("每批片段数"), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText("数据加载进程"), { target: { value: "0" } });
    fireEvent.change(screen.getByLabelText("CPU 线程数"), { target: { value: "6" } });
    fireEvent.change(screen.getByLabelText("GPU 编号"), { target: { value: "1" } });
    await user.click(screen.getByLabelText("节省显存"));
    await user.upload(screen.getByLabelText("训练片段"), Array.from({ length: 6 }, () => pcm16Wav()));
    await screen.findByLabelText("片段 5 语言");
    expect(screen.queryByLabelText("片段 6 语言")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "训练片段下一页" }));
    expect(screen.getByLabelText("片段 6 语言")).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText("片段 6 语言"), "en");
    await user.click(screen.getByRole("button", { name: "收起片段编辑" }));
    expect(screen.queryByLabelText("片段 6 语言")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "展开片段编辑" }));
    expect(screen.getByLabelText("片段 6 语言")).toHaveValue("en");
    await user.click(screen.getByRole("tab", { name: /训练记录/ }));
    expect(screen.queryByLabelText("训练名称")).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "新建训练" }));
    expect(screen.getByLabelText("训练名称")).toHaveValue("自定义训练");
    expect(screen.getByLabelText("片段 6 语言")).toHaveValue("en");
    await user.click(screen.getByRole("button", { name: "开始训练" }));
    await waitFor(() => expect(deps.client.create).toHaveBeenCalledWith(expect.objectContaining({ performance: { batch_size: 3, data_workers: 0, cpu_threads: 6, gpu_index: 1, low_memory: false }, clips: expect.arrayContaining([{ text: "", language: "en" }]) }), expect.any(Array), expect.any(AbortSignal)));
  });

  it("训练记录分页，同一参考音色的七次训练只展示一张卡片，版本详情默认收起", async () => {
    const deps = setup(7), user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    expect(screen.queryByRole("list", { name: "训练任务" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: /训练记录/ }));
    const list = screen.getByRole("list", { name: "训练任务" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(5);
    expect(screen.getByText("完成详情 6")).not.toBeVisible();
    fireEvent.click(screen.getAllByText("查看任务详情", { selector: "summary" })[0]);
    expect(screen.getByText("完成详情 6")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "训练记录下一页" }));
    expect(within(list).getAllByRole("listitem")).toHaveLength(2);
    await user.click(screen.getByRole("tab", { name: /已训练音色/ }));
    expect(screen.queryByRole("list", { name: "训练任务" })).not.toBeInTheDocument();
    const voices = screen.getByRole("list", { name: "已训练音色" });
    expect(within(voices).getAllByRole("listitem")).toHaveLength(1);
    expect(screen.getByRole("tab", { name: "已训练音色 (1)" })).toBeVisible();
    expect(within(screen.getByLabelText("选择已保存音色")).getAllByRole("option")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "已训练音色下一页" })).toBeDisabled();
    expect(screen.getByLabelText("温柔旁白 的训练版本")).not.toBeVisible();
    await user.click(screen.getByText("查看训练版本（7）"));
    expect(screen.getByLabelText("温柔旁白 的训练版本")).toHaveValue("job-6");
    await user.selectOptions(screen.getByLabelText("温柔旁白 的训练版本"), "job-0");
    expect(screen.getByRole("button", { name: "删除版本 已训练音色 0" })).toBeVisible();
  });

  it("模型启停独立于选用音色，关闭后禁用试听但保留版本", async () => {
    const deps = setup(1), user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    await user.click(await screen.findByRole("button", { name: "启用语音模型" }));
    await user.click(await screen.findByRole("button", { name: "关闭语音模型" }));
    await screen.findByRole("button", { name: "启用语音模型" });
    expect(deps.client.setModelsEnabled).toHaveBeenNthCalledWith(1, true, expect.any(AbortSignal));
    expect(deps.client.setModelsEnabled).toHaveBeenNthCalledWith(2, false, expect.any(AbortSignal));
    expect(deps.client.activate).not.toHaveBeenCalled();
    expect(deps.client.delete).not.toHaveBeenCalled();
    await user.click(screen.getByRole("tab", { name: /已训练音色/ }));
    expect(screen.getByRole("button", { name: "生成版本试听" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "删除版本 已训练音色 0" })).toBeEnabled();
  });

  it("TTS 不可用时仍能浏览训练记录，模型开关显示不可用", async () => {
    const deps = setup(1), user = userEvent.setup();
    vi.mocked(deps.client.modelStatus).mockRejectedValue(new Error("TTS 未启动"));
    render(<TrainingPanel {...deps} />);
    await screen.findByText("TTS 未启动");
    expect(screen.getByRole("button", { name: "启用语音模型" })).toBeDisabled();
    await user.click(screen.getByRole("tab", { name: /训练记录/ }));
    expect(screen.getByText("历史训练 0")).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});

it("关闭模型时可选用保存的音色，选择动作不会加载模型", async () => {
  const deps = setup(2), user = userEvent.setup();
  render(<TrainingPanel {...deps} />);
  await screen.findByText("训练就绪");
  await user.click(screen.getByRole("tab", { name: /已训练音色/ }));
  await user.selectOptions(screen.getByLabelText("选择已保存音色"), "voice-1");
  await user.click(screen.getByRole("button", { name: "确认所选音色" }));
  await waitFor(() => expect(deps.client.activate).toHaveBeenCalledWith("job-1", expect.any(AbortSignal)));
  expect(deps.client.setModelsEnabled).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "启用语音模型" })).toBeEnabled();
});

it("旧引擎禁止模型开关但保留试听入口，并提示重启升级", async () => {
  const deps = setup(1), user = userEvent.setup();
  vi.mocked(deps.client.modelStatus).mockResolvedValue({ supported: false, state: "unsupported", message: "旧引擎" });
  render(<TrainingPanel {...deps} />);
  await screen.findByText("旧引擎");
  expect(screen.getByRole("button", { name: "启用语音模型" })).toBeDisabled();
  expect(screen.getByText(/当前引擎不支持独立启停/)).toBeVisible();
  await user.click(screen.getByRole("tab", { name: /已训练音色/ }));
  expect(screen.getByRole("button", { name: "生成版本试听" })).toBeEnabled();
});
