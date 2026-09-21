import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { VoicePanel } from "./VoicePanel";
import { pcm16Wav, resourceSnapshot } from "../../test/resource-fixtures";
import type { TrainingTranscription } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";

function setup() {
  const controller = {
    snapshot: resourceSnapshot(), pendingAction: null, voicePreview: null, voiceDeleteRetries: [],
    selectVoice: vi.fn(), deleteVoice: vi.fn(), previewVoice: vi.fn(),
    uploadVoice: vi.fn().mockResolvedValue(null),
  };
  const transcriber = { transcribe: vi.fn<TrainingClient["transcribe"]>().mockResolvedValue({ text: "这是自动识别的参考录音", language: "zh" }) };
  return { controller, transcriber };
}

describe("参考音频自动转写", () => {
  it("选择音频后自动回填，并将同一音频与文本一起上传", async () => {
    const user = userEvent.setup();
    const props = setup();
    render(<VoicePanel {...props} />);
    const file = pcm16Wav();
    await user.type(screen.getByLabelText("音色名称"), "自动音色");
    await user.upload(screen.getByLabelText("参考音频"), file);
    await waitFor(() => expect(screen.getByLabelText("参考文本")).toHaveValue("这是自动识别的参考录音"));
    expect(props.transcriber.transcribe).toHaveBeenCalledWith(file, "zh", expect.any(AbortSignal));
    await user.click(screen.getByRole("button", { name: "上传音色" }));
    expect(props.controller.uploadVoice).toHaveBeenCalledWith({ name: "自动音色", language: "zh", reference_text: "这是自动识别的参考录音" }, file);
  });

  it("保留预先填写的文本以及识别等待期间的手工修改", async () => {
    const user = userEvent.setup();
    const props = setup();
    let resolve!: (value: TrainingTranscription) => void;
    props.transcriber.transcribe.mockImplementation(() => new Promise(r => { resolve = r; }));
    render(<VoicePanel {...props} />);
    await user.type(screen.getByLabelText("参考文本"), "手工原文");
    await user.upload(screen.getByLabelText("参考音频"), pcm16Wav());
    await screen.findByText(/音频有效/);
    expect(props.transcriber.transcribe).not.toHaveBeenCalled();
    await user.clear(screen.getByLabelText("参考文本"));
    await user.click(screen.getByRole("button", { name: "提取参考文本" }));
    await waitFor(() => expect(props.transcriber.transcribe).toHaveBeenCalledOnce());
    await user.type(screen.getByLabelText("参考文本"), "等待时填写的原文");
    await act(async () => resolve({ text: "迟到的识别结果", language: "zh" }));
    expect(screen.getByLabelText("参考文本")).toHaveValue("等待时填写的原文");
  });

  it("换文件后丢弃旧识别结果，换语言时重新提取自动生成的文本", async () => {
    const user = userEvent.setup();
    const props = setup();
    let resolveOld!: (value: TrainingTranscription) => void;
    props.transcriber.transcribe.mockImplementationOnce(() => new Promise(r => { resolveOld = r; }))
      .mockResolvedValueOnce({ text: "新文件原文", language: "zh" })
      .mockResolvedValueOnce({ text: "New recording", language: "en" });
    render(<VoicePanel {...props} />);
    await user.upload(screen.getByLabelText("参考音频"), pcm16Wav());
    await waitFor(() => expect(props.transcriber.transcribe).toHaveBeenCalledOnce());
    const signal = props.transcriber.transcribe.mock.calls[0][2];
    await user.upload(screen.getByLabelText("参考音频"), new File([pcm16Wav()], "new.wav", { type: "audio/wav" }));
    await waitFor(() => expect(screen.getByLabelText("参考文本")).toHaveValue("新文件原文"));
    expect(signal?.aborted).toBe(true);
    await act(async () => resolveOld({ text: "旧文件原文", language: "zh" }));
    expect(screen.getByLabelText("参考文本")).toHaveValue("新文件原文");
    await user.selectOptions(screen.getByLabelText("参考文本语言"), "en");
    await waitFor(() => expect(screen.getByLabelText("参考文本")).toHaveValue("New recording"));
  });

  it("转写失败可重试；自动识别语言允许手工填写并给出转写提示", async () => {
    const user = userEvent.setup();
    const props = setup();
    props.transcriber.transcribe.mockRejectedValueOnce(new Error("请先下载识别模型"));
    render(<VoicePanel {...props} />);
    await user.upload(screen.getByLabelText("参考音频"), pcm16Wav());
    expect(await screen.findByRole("alert")).toHaveTextContent("请先下载识别模型");
    await user.click(screen.getByRole("button", { name: "提取参考文本" }));
    await waitFor(() => expect(screen.getByLabelText("参考文本")).toHaveValue("这是自动识别的参考录音"));
    await user.selectOptions(screen.getByLabelText("参考文本语言"), "auto");
    expect(screen.getByText(/自动转写前请选择录音的具体语言/)).toBeVisible();
    expect(screen.getByRole("button", { name: "提取参考文本" })).toBeDisabled();
    expect(props.transcriber.transcribe).toHaveBeenCalledTimes(2);
  });
});
