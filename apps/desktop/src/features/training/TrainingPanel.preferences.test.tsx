import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";

function setup(baseUrl = "http://127.0.0.1:19600") {
  const client: TrainingClient = {
    modelStatus: vi.fn().mockResolvedValue({ supported: true, state: "unloaded", message: "模型已关闭" }),
    setModelsEnabled: vi.fn(),
    snapshot: vi.fn().mockResolvedValue({ enabled: true, busy: false, jobs: [], versions: [] }),
    create: vi.fn(), transcribe: vi.fn(), cancel: vi.fn(), delete: vi.fn(), save: vi.fn(), activate: vi.fn(), audition: vi.fn(), measure: vi.fn(),
    preset: vi.fn().mockResolvedValue({ mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" }),
  };
  const resources = { baseUrl, getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources };
}

beforeEach(() => localStorage.clear());
afterEach(() => { vi.restoreAllMocks(); localStorage.clear(); });

describe("训练面板本地偏好", () => {
  it("重新打开恢复训练参数与试听文本，并要求重新选择素材和确认", async () => {
    const user = userEvent.setup();
    const deps = setup();
    const view = render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "7" } });
    fireEvent.change(screen.getByLabelText("每批片段数"), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText("数据加载进程"), { target: { value: "0" } });
    fireEvent.change(screen.getByLabelText("CPU 线程数"), { target: { value: "6" } });
    fireEvent.change(screen.getByLabelText("GPU 编号"), { target: { value: "2" } });
    await user.click(screen.getByLabelText("节省显存"));
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.type(screen.getByLabelText("训练名称"), "仅本次训练");
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    await user.upload(screen.getByLabelText("训练片段"), [pcm16Wav(), pcm16Wav()]);
    fireEvent.change(await screen.findByLabelText("片段 1 文本"), { target: { value: "第一段" } });
    fireEvent.change(screen.getByLabelText("片段 2 文本"), { target: { value: "第二段" } });
    await user.click(screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致"));
    expect(screen.getByRole("button", { name: "开始训练" })).toBeEnabled();
    await user.click(screen.getByRole("tab", { name: "离线设置" }));
    fireEvent.change(screen.getByLabelText("试听与测量文本"), { target: { value: "欢迎回来，今天也一起加油。" } });
    view.unmount();

    const reopened = setup();
    render(<TrainingPanel {...reopened} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(7);
    expect(screen.getByLabelText("性能预设")).toHaveValue("custom");
    expect(screen.getByLabelText("每批片段数")).toHaveValue(3);
    expect(screen.getByLabelText("数据加载进程")).toHaveValue(0);
    expect(screen.getByLabelText("CPU 线程数")).toHaveValue(6);
    expect(screen.getByLabelText("GPU 编号")).toHaveValue(2);
    expect(screen.getByLabelText("节省显存")).not.toBeChecked();
    expect(screen.getByLabelText("输入并校对文本")).toBeChecked();
    expect(screen.getByLabelText("训练名称")).toHaveValue("");
    expect(screen.getByLabelText("训练音色")).toHaveValue("");
    expect(screen.queryByLabelText("片段 1 文本")).not.toBeInTheDocument();
    expect(screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致")).not.toBeChecked();
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
    await user.click(screen.getByRole("tab", { name: /已训练音色/ }));
    expect(screen.getByLabelText("试听与测量文本")).toHaveValue("欢迎回来，今天也一起加油。");
    expect(reopened.client.create).not.toHaveBeenCalled();
    expect(reopened.client.setModelsEnabled).not.toHaveBeenCalled();
    expect(reopened.client.measure).not.toHaveBeenCalled();
    expect(reopened.client.audition).not.toHaveBeenCalled();
  });

  it("更换服务时隔离偏好，返回原服务仍恢复对应预设", async () => {
    const first = setup("http://127.0.0.1:19600");
    const second = setup("http://127.0.0.1:19601");
    const view = render(<TrainingPanel {...first} />);
    await screen.findByText("训练就绪");
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "4" } });
    fireEvent.change(screen.getByLabelText("性能预设"), { target: { value: "balanced" } });
    view.rerender(<TrainingPanel {...second} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(1);
    expect(screen.getByLabelText("性能预设")).toHaveValue("low");
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "9" } });
    view.rerender(<TrainingPanel {...first} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(4);
    expect(screen.getByLabelText("性能预设")).toHaveValue("balanced");
    expect(screen.getByLabelText("每批片段数")).toHaveValue(2);
    expect(screen.getByLabelText("CPU 线程数")).toHaveValue(4);
  });

  it("小数轮次取整后，后续性能与文本模式修改仍可恢复", async () => {
    const view = render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "2.5" } });
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(2);
    fireEvent.change(screen.getByLabelText("性能预设"), { target: { value: "balanced" } });
    fireEvent.click(screen.getByLabelText("输入并校对文本"));
    view.unmount();
    render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(2);
    expect(screen.getByLabelText("性能预设")).toHaveValue("balanced");
    expect(screen.getByLabelText("每批片段数")).toHaveValue(2);
    expect(screen.getByLabelText("输入并校对文本")).toBeChecked();
  });

  it("浏览器禁止本地存储时仍可编辑训练配置", async () => {
    vi.spyOn(window, "localStorage", "get").mockImplementation(() => { throw new DOMException("Storage blocked", "SecurityError"); });
    render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "5" } });
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(5);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("存储写入失败时保留已恢复的值并允许继续编辑", async () => {
    const view = render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "7" } });
    view.unmount();
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new DOMException("Storage full", "QuotaExceededError"); });
    render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(7);
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "5" } });
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(5);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  const saved = { epochs: 6, performancePreset: "custom", performance: { batch_size: 3, data_workers: 0, cpu_threads: 6, gpu_index: 2, low_memory: false }, textMode: true, auditionText: "保存的试听文本" };
  it.each([
    ["损坏 JSON", "{"],
    ["空对象", "{}"],
    ["数组", "[]"],
    ["过多轮次", JSON.stringify({ ...saved, epochs: 21 })],
    ["小数轮次", JSON.stringify({ ...saved, epochs: 1.5 })],
    ["无效预设", JSON.stringify({ ...saved, performancePreset: "unknown" })],
    ["字符串开关", JSON.stringify({ ...saved, textMode: "false" })],
    ["超长试听文本", JSON.stringify({ ...saved, auditionText: "长".repeat(501) })],
    ["控制字符", JSON.stringify({ ...saved, auditionText: "文字\u0000" })],
    ...[
      { batch_size: 0 }, { batch_size: 17 }, { data_workers: -1 }, { data_workers: 9 },
      { cpu_threads: 0 }, { cpu_threads: 17 }, { gpu_index: -1 }, { gpu_index: 16 }, { low_memory: "false" },
    ].map(change => [JSON.stringify(change), JSON.stringify({ ...saved, performance: { ...saved.performance, ...change } })]),
  ])("忽略无效偏好（%s），之后的合法修改仍可恢复", async (_name, raw) => {
    localStorage.setItem("meowlive.training-preferences.v1:http://127.0.0.1:19600", raw);
    const view = render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(1);
    expect(screen.getByLabelText("性能预设")).toHaveValue("low");
    expect(screen.getByLabelText("每批片段数")).toHaveValue(1);
    expect(screen.getByLabelText("输入并校对文本")).not.toBeChecked();
    fireEvent.click(screen.getByRole("tab", { name: "离线设置" }));
    expect(screen.getByLabelText("试听与测量文本")).toHaveValue("你好，欢迎来到直播间，希望你今天过得愉快。");
    fireEvent.click(screen.getByRole("tab", { name: "新建训练" }));
    fireEvent.change(screen.getByLabelText("GPT / SoVITS 轮次"), { target: { value: "8" } });
    view.unmount();
    render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    expect(screen.getByLabelText("GPT / SoVITS 轮次")).toHaveValue(8);
  });
});
