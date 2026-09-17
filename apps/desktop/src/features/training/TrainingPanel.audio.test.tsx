import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import * as audioService from "../../services/audio";
import { pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";

function setup() {
  const client: TrainingClient = { modelStatus: vi.fn().mockResolvedValue({ supported: true, state: "loaded", message: "模型已启用" }), setModelsEnabled: vi.fn(),
    snapshot: vi.fn().mockResolvedValue({ enabled: true, busy: false, jobs: [], versions: [] }),
    transcribe: vi.fn(), create: vi.fn().mockResolvedValue({}), cancel: vi.fn(), delete: vi.fn(), save: vi.fn(), activate: vi.fn(), audition: vi.fn(), measure: vi.fn(),
    preset: vi.fn().mockResolvedValue({ mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "本地预设未验证" }),
  };
  const resources = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources };
}

function prepared(file: File) {
  return { file, info: { durationMs: 4000, sampleRate: 8000, channels: 1 } };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

describe("训练音频导入", () => {
  it("保留 MP3 与 FLAC 原文件名供文本核对，提交转换后的 WAV", async () => {
    const first = new File(["mp3"], "第一句.mp3", { type: "audio/mpeg" });
    const second = new File(["flac"], "第二句.flac", { type: "audio/flac" });
    const waves = [pcm16Wav(), pcm16Wav()];
    vi.spyOn(audioService, "prepareAudioFile").mockResolvedValueOnce(prepared(waves[0]!)).mockResolvedValueOnce(prepared(waves[1]!));
    const user = userEvent.setup();
    const deps = setup();
    render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    fireEvent.click(screen.getByLabelText("输入并校对文本"));
    await user.type(screen.getByLabelText("训练名称"), "音色训练");
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    await user.upload(screen.getByLabelText("训练片段"), [first, second]);
    await user.type(await screen.findByLabelText("片段 1 文本"), "第一句文本");
    await user.type(screen.getByLabelText("片段 2 文本"), "第二句文本");
    expect(screen.getByRole("group", { name: "第一句.mp3" })).toBeVisible();
    expect(screen.getByRole("group", { name: "第二句.flac" })).toBeVisible();
    await user.click(screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致"));
    await user.click(screen.getByRole("button", { name: "开始训练" }));
    await waitFor(() => expect(deps.client.create).toHaveBeenCalledWith(expect.objectContaining({ clips: [
      { text: "第一句文本", language: "zh" }, { text: "第二句文本", language: "zh" },
    ] }), waves, expect.any(AbortSignal)));
    const submitted = vi.mocked(deps.client.create).mock.calls[0]![1];
    expect(submitted[0]).toBe(waves[0]);
    expect(submitted[1]).toBe(waves[1]);
    expect(submitted).not.toContain(first);
  });

  it("损坏片段显示原文件名并阻止审核与提交", async () => {
    const user = userEvent.setup();
    const deps = setup();
    render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    fireEvent.click(screen.getByLabelText("输入并校对文本"));
    await user.type(screen.getByLabelText("训练名称"), "音色训练");
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    fireEvent.change(screen.getByLabelText("训练片段"), { target: { files: [new File(["broken"], "损坏录音.wav", { type: "audio/wav" }), pcm16Wav()] } });

    expect(await screen.findByRole("alert")).toHaveTextContent("损坏录音.wav");
    expect(screen.queryByLabelText("片段 1 文本")).not.toBeInTheDocument();
    expect(screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致")).toBeDisabled();
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
    expect(deps.client.create).not.toHaveBeenCalled();
  });

  it("准备期间禁止审核和提交，可重新选择且旧结果不会覆盖新片段", async () => {
    const first = new File(["old"], "旧片段.mp3");
    const oldSecond = new File(["old 2"], "旧第二段.mp3");
    const newer = [new File(["new 1"], "新第一段.mp3"), new File(["new 2"], "新第二段.mp3")];
    const pending = deferred<ReturnType<typeof prepared>>();
    const wave = pcm16Wav();
    const prepare = vi.spyOn(audioService, "prepareAudioFile").mockImplementation(file => file === first ? pending.promise : Promise.resolve(prepared(wave)));
    const user = userEvent.setup();
    const deps = setup();
    render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    fireEvent.click(screen.getByLabelText("输入并校对文本"));
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    const input = screen.getByLabelText("训练片段");
    fireEvent.change(input, { target: { files: [first, oldSecond] } });
    expect(screen.getByText(/正在准备音频/)).toBeVisible();
    expect(screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致")).toBeDisabled();
    expect(screen.getByRole("button", { name: /开始训练|提取空白片段文本/ })).toBeDisabled();
    expect(input).toBeEnabled();
    expect(prepare).toHaveBeenCalledTimes(1);

    fireEvent.change(input, { target: { files: newer } });
    await screen.findByRole("group", { name: "新第一段.mp3" });
    await act(async () => { pending.resolve(prepared(wave)); await pending.promise; });
    expect(screen.getByRole("group", { name: "新第二段.mp3" })).toBeVisible();
    expect(screen.queryByRole("group", { name: "旧片段.mp3" })).not.toBeInTheDocument();
    expect(prepare.mock.calls.map(([file]) => file)).not.toContain(oldSecond);
    expect(screen.queryByText(/正在准备音频/)).not.toBeInTheDocument();
  });

  it("按转换后大小拒绝超过 32 MiB 的训练素材", async () => {
    const wave = pcm16Wav({ seconds: 10, sampleRate: 48000, channels: 2 });
    vi.spyOn(audioService, "prepareAudioFile").mockResolvedValue(prepared(wave));
    const files = Array.from({ length: 18 }, (_, index) => new File(["small mp3"], `${index}.mp3`));
    render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    fireEvent.click(screen.getByLabelText("输入并校对文本"));
    fireEvent.change(screen.getByLabelText("训练片段"), { target: { files } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/转换.*32 MiB/);
    expect(screen.queryByLabelText("片段 1 文本")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
  });

  it("切换服务取消旧导入并恢复准备状态，旧结果不回填", async () => {
    const pending = deferred<ReturnType<typeof prepared>>();
    const wave = pcm16Wav();
    vi.spyOn(audioService, "prepareAudioFile").mockReturnValueOnce(pending.promise).mockResolvedValue(prepared(wave));
    const deps = setup();
    const view = render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    fireEvent.change(screen.getByLabelText("训练片段"), { target: { files: [new File(["old"], "旧导入.mp3"), wave] } });
    expect(screen.getByText(/正在准备音频/)).toBeVisible();
    view.rerender(<TrainingPanel client={setup().client} resources={deps.resources} />);
    await waitFor(() => expect(screen.queryByText(/正在准备音频/)).not.toBeInTheDocument());
    await act(async () => { pending.resolve(prepared(wave)); await pending.promise; });
    expect(screen.queryByRole("group", { name: "旧导入.mp3" })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("训练片段"), { target: { files: [wave, wave] } });
    await screen.findByLabelText("片段 2 语言");
  });

  it("准备失败后允许重新选择有效片段，清除上一次导入错误", async () => {
    const wave = pcm16Wav();
    vi.spyOn(audioService, "prepareAudioFile").mockRejectedValueOnce(new Error("音频解码失败")).mockResolvedValue(prepared(wave));
    render(<TrainingPanel {...setup()} />);
    await screen.findByText("训练就绪");
    fireEvent.click(screen.getByLabelText("输入并校对文本"));
    const input = screen.getByLabelText("训练片段");
    fireEvent.change(input, { target: { files: [new File(["bad"], "坏片段.mp3"), wave] } });
    expect(await screen.findByRole("alert")).toHaveTextContent("坏片段.mp3");
    fireEvent.change(input, { target: { files: [wave, wave] } });
    await screen.findByLabelText("片段 1 文本");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
