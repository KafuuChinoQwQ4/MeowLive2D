import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";

function setup() {
  const client: TrainingClient = { modelStatus: vi.fn().mockResolvedValue({ supported: true, state: "loaded", message: "模型已启用" }), setModelsEnabled: vi.fn(),
    snapshot: vi.fn().mockResolvedValue({ enabled: true, busy: false, jobs: [], versions: [] }),
    create: vi.fn().mockResolvedValue({}), transcribe: vi.fn().mockResolvedValue({ text: "提取的文本", language: "zh" }),
    cancel: vi.fn(), delete: vi.fn(), save: vi.fn(), activate: vi.fn(), audition: vi.fn(), measure: vi.fn(),
    preset: vi.fn().mockResolvedValue({ mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "未验证" }),
  };
  const resources = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources };
}
async function configure(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByText("训练就绪");
  await user.type(screen.getByLabelText("训练名称"), "新的训练");
  await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
  await user.upload(screen.getByLabelText("训练片段"), [pcm16Wav(), pcm16Wav()]);
  await screen.findByLabelText("片段 2 语言");
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { resolve, promise };
}
const reviewLabel = "我已逐片听取并核对文本，素材与所选音色一致";

describe("训练文本模式", () => {
  it("默认声音模式无需文本或审核即可提交，并明确发送声音模式与空文本", async () => {
    const user = userEvent.setup(), deps = setup();
    render(<TrainingPanel {...deps} />);
    await configure(user);
    expect(screen.queryByLabelText("片段 1 文本")).not.toBeInTheDocument();
    expect(screen.queryByLabelText(reviewLabel)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "开始训练" }));
    await waitFor(() => expect(deps.client.create).toHaveBeenCalledWith(expect.objectContaining({
      text_mode: "audio_only", reviewed: false, clips: [{ text: "", language: "zh" }, { text: "", language: "zh" }],
    }), expect.any(Array), expect.any(AbortSignal)));
    expect(deps.client.transcribe).not.toHaveBeenCalled();
  });

  it("文字模式提交时仅提取空白片段，保留手工文本且必须校对确认后才能训练", async () => {
    const user = userEvent.setup(), deps = setup();
    render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.type(screen.getByLabelText("片段 1 文本"), "手工第一句");
    expect(screen.getByLabelText(reviewLabel)).toBeDisabled();
    fireEvent.submit(screen.getByRole("button", { name: "提取空白片段文本" }).closest("form")!);
    await waitFor(() => expect(screen.getByLabelText("片段 2 文本")).toHaveValue("提取的文本"));
    expect(screen.getByLabelText("片段 1 文本")).toHaveValue("手工第一句");
    expect(deps.client.transcribe).toHaveBeenCalledTimes(1);
    expect(deps.client.transcribe).toHaveBeenCalledWith(expect.any(File), "zh", expect.any(AbortSignal));
    expect(deps.client.create).not.toHaveBeenCalled();
    const result = await screen.findByRole("status", { name: "文本提取成功" });
    expect(result).toHaveTextContent("1 个片段");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    const start = screen.getByRole("button", { name: "开始训练" });
    expect(start).toBeDisabled();
    fireEvent.submit(start.closest("form")!);
    expect(deps.client.create).not.toHaveBeenCalled();
    await user.type(screen.getByLabelText("片段 2 文本"), "已校对");
    await user.click(screen.getByLabelText(reviewLabel));
    await user.click(start);
    await waitFor(() => expect(deps.client.create).toHaveBeenCalledWith(expect.objectContaining({
      text_mode: "reviewed_text", reviewed: true, clips: [{ text: "手工第一句", language: "zh" }, { text: "提取的文本已校对", language: "zh" }],
    }), expect.any(Array), expect.any(AbortSignal)));
  });

  it("转写失败显示对应片段并可重试，重试只处理仍空白的片段", async () => {
    const user = userEvent.setup(), deps = setup();
    vi.mocked(deps.client.transcribe).mockRejectedValueOnce(new Error("识别服务离线"));
    render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("识别服务离线");
    await waitFor(() => expect(screen.getByLabelText("片段 2 文本")).toHaveValue("提取的文本"));
    expect(screen.getByLabelText("片段 1 文本")).toHaveValue("");
    const result = await screen.findByRole("dialog", { name: "文本提取失败" });
    expect(result).toHaveTextContent("1 个片段成功，1 个片段失败");
    expect(result).toHaveTextContent("片段 1");
    expect(result).toHaveTextContent("手工填写");
    await user.click(within(result).getByRole("button", { name: "知道了" }));
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    await waitFor(() => expect(screen.getByLabelText("片段 1 文本")).toHaveValue("提取的文本"));
    expect(deps.client.transcribe).toHaveBeenCalledTimes(3);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
  });

  it("转写失败后允许手工补齐并确认，关闭文本模式后不会提交隐藏的文本", async () => {
    const user = userEvent.setup(), deps = setup();
    vi.mocked(deps.client.transcribe).mockRejectedValue(new Error("识别服务离线"));
    render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.type(screen.getByLabelText("片段 1 文本"), "原有手工文本");
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    await screen.findByRole("alert");
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "知道了" }));
    await user.type(screen.getByLabelText("片段 2 文本"), "补齐文本");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.click(screen.getByLabelText(reviewLabel));
    expect(screen.getByRole("button", { name: "开始训练" })).toBeEnabled();
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.click(screen.getByRole("button", { name: "开始训练" }));
    await waitFor(() => expect(deps.client.create).toHaveBeenCalledWith(expect.objectContaining({
      text_mode: "audio_only", reviewed: false, clips: [{ text: "", language: "zh" }, { text: "", language: "zh" }],
    }), expect.any(Array), expect.any(AbortSignal)));
  });

  it("转写尚未返回时填写的文字不被识别结果覆盖", async () => {
    const user = userEvent.setup(), deps = setup(), pending = deferred<{ text: string; language: string }>();
    vi.mocked(deps.client.transcribe).mockReturnValueOnce(pending.promise);
    render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.type(screen.getByLabelText("片段 2 文本"), "第二句");
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    await user.type(screen.getByLabelText("片段 1 文本"), "临时手工填写");
    await act(async () => { pending.resolve({ text: "过时的识别内容", language: "zh" }); await pending.promise; });
    expect(screen.getByLabelText("片段 1 文本")).toHaveValue("临时手工填写");
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
  });

  it.each(["模式", "文件", "语言", "卸载"])("变更%s会取消旧转写且忽略晚到结果", async change => {
    const user = userEvent.setup(), deps = setup(), pending = deferred<{ text: string; language: string }>();
    vi.mocked(deps.client.transcribe).mockReturnValueOnce(pending.promise);
    const view = render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    const signal = vi.mocked(deps.client.transcribe).mock.calls[0]![2];
    if (change === "模式") {
      await user.click(screen.getByLabelText("输入并校对文本"));
      await user.click(screen.getByLabelText("输入并校对文本"));
    } else if (change === "文件") {
      await user.upload(screen.getByLabelText("训练片段"), [pcm16Wav(), pcm16Wav()]);
      await screen.findByLabelText("片段 1 文本");
    } else if (change === "语言") {
      await user.selectOptions(screen.getByLabelText("片段 1 语言"), "en");
    } else view.unmount();
    expect(signal?.aborted).toBe(true);
    await act(async () => { pending.resolve({ text: "过时的识别内容", language: "zh" }); await pending.promise; });
    if (change !== "卸载") expect(screen.getByLabelText("片段 1 文本")).toHaveValue("");
    expect(deps.client.transcribe).toHaveBeenCalledTimes(1);
    expect(deps.client.create).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("多个片段失败汇总为一个弹窗，保留每段原因且关闭后不连续弹出", async () => {
    const user = userEvent.setup(), deps = setup();
    vi.mocked(deps.client.transcribe)
      .mockRejectedValueOnce(new Error("识别模型不可用"))
      .mockRejectedValueOnce(new Error("录音中未检测到语音"));
    render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    const result = await screen.findByRole("dialog", { name: "文本提取失败" });
    expect(result).toHaveTextContent("0 个片段成功，2 个片段失败");
    expect(result).toHaveTextContent("识别模型不可用");
    expect(result).toHaveTextContent("录音中未检测到语音");
    await user.click(within(result).getByRole("button", { name: "知道了" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "提取空白片段文本" })).toHaveFocus();
  });

  it("切换服务时取消旧转写并恢复可提取状态", async () => {
    const user = userEvent.setup(), deps = setup(), pending = deferred<{ text: string; language: string }>();
    vi.mocked(deps.client.transcribe).mockReturnValueOnce(pending.promise);
    const view = render(<TrainingPanel {...deps} />);
    await configure(user);
    await user.click(screen.getByLabelText("输入并校对文本"));
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    const signal = vi.mocked(deps.client.transcribe).mock.calls[0]![2];
    const replacement = setup().client;
    view.rerender(<TrainingPanel client={replacement} resources={deps.resources} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "提取空白片段文本" })).toBeEnabled());
    expect(signal?.aborted).toBe(true);
    await act(async () => { pending.resolve({ text: "旧服务结果", language: "zh" }); await pending.promise; });
    expect(screen.getByLabelText("片段 1 文本")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: "提取空白片段文本" }));
    await waitFor(() => expect(screen.getByLabelText("片段 1 文本")).toHaveValue("提取的文本"));
    expect(replacement.transcribe).toHaveBeenCalledTimes(2);
    expect(screen.getByLabelText(reviewLabel)).not.toBeChecked();
  });
});
