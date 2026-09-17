import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";
import { pcm16Wav, resourceSnapshot, voice } from "../../test/resource-fixtures";
import { ResourcesPanel } from "../../app/resources";
import { validateVoiceWav } from "../../services/audio/wav";

afterEach(() => vi.unstubAllGlobals());

function clients(snapshot = resourceSnapshot()) {
  const resourceClient: ResourcesClient = {
    baseUrl: "http://localhost:19600",
    getSnapshot: vi.fn().mockResolvedValue(snapshot),
    createVoice: vi.fn().mockResolvedValue(snapshot),
    selectVoice: vi.fn().mockResolvedValue(snapshot),
    deleteVoice: vi.fn().mockResolvedValue(snapshot),
    deleteCharacter: vi.fn().mockResolvedValue(snapshot),
    saveCharacter: vi.fn().mockResolvedValue(snapshot),
    selectCharacter: vi.fn().mockResolvedValue(snapshot),
    previewCharacter: vi.fn().mockResolvedValue(snapshot),
    desktop: vi.fn().mockResolvedValue({ type: "models", models: [] }),
  };
  const speechClient: ServerClient = {
    baseUrl: "http://localhost:19600",
    getStatus: vi.fn(),
    stop: vi.fn(),
    submitSpeech: vi.fn().mockResolvedValue({
      id: "speech-preview", generation: 1, text: voice().reference_text,
      voice_id: "voice-1", status: "queued", error: null,
    }),
  };
  return { resourceClient, speechClient };
}

describe("音色管理", () => {
  it("确认后删除音色并清空列表，取消时保留", async () => {
    const user = userEvent.setup();
    const pair = clients();
    const confirm = vi.fn().mockReturnValue(false);
    vi.stubGlobal("confirm", confirm);
    pair.resourceClient.deleteVoice = vi.fn().mockResolvedValue(resourceSnapshot({
      voices: [], characters: [], active_voice_id: "", active_character_id: null, default_voice_available: false,
    }));
    render(<ResourcesPanel {...pair} />);
    const button = await screen.findByRole("button", { name: "删除音色 温柔旁白" });
    await user.click(button);
    expect(pair.resourceClient.deleteVoice).not.toHaveBeenCalled();
    confirm.mockReturnValue(true);
    await user.click(button);
    expect(pair.resourceClient.deleteVoice).toHaveBeenCalledWith({ id: "voice-1" }, expect.any(AbortSignal));
    await waitFor(() => expect(screen.queryByText("温柔旁白")).not.toBeInTheDocument());
    expect(screen.queryByText("当前音色")).not.toBeInTheDocument();
  });

  it("删除失败显示原因并保留音色", async () => {
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(true));
    const pair = clients();
    pair.resourceClient.deleteVoice = vi.fn().mockRejectedValue(new Error("请先删除关联训练版本"));
    render(<ResourcesPanel {...pair} />);
    await userEvent.setup().click(await screen.findByRole("button", { name: "删除音色 温柔旁白" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("请先删除关联训练版本");
    expect(screen.getByText("温柔旁白", { selector: "strong" })).toBeVisible();
  });
  it("文件清理失败后保留重试入口，即使音色记录已消失", async () => {
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(true));
    const user = userEvent.setup();
    const pair = clients();
    const empty = resourceSnapshot({ voices: [], characters: [], active_voice_id: "", active_character_id: null });
    pair.resourceClient.getSnapshot = vi.fn().mockResolvedValueOnce(resourceSnapshot()).mockResolvedValue(empty);
    pair.resourceClient.deleteVoice = vi.fn().mockRejectedValueOnce(new Error("音色配置已删除，但参考音频清理失败，请重试"))
      .mockResolvedValueOnce(empty);
    render(<ResourcesPanel {...pair} />);
    await user.click(await screen.findByRole("button", { name: "删除音色 温柔旁白" }));
    await user.click(await screen.findByRole("button", { name: "重试清理参考音频" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "重试清理参考音频" })).not.toBeInTheDocument());
    expect(pair.resourceClient.deleteVoice).toHaveBeenCalledTimes(2);
  });
  it("选择 MP3 后上传实际转换出的 WAV 文件", async () => {
    vi.stubGlobal("OfflineAudioContext", class {
      async decodeAudioData() {
        return { sampleRate: 48_000, length: 192_000, numberOfChannels: 1,
          getChannelData: () => new Float32Array(192_000).fill(0.25) };
      }
    });
    const user = userEvent.setup();
    const pair = clients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });
    await user.type(screen.getByLabelText("音色名称"), "MP3 音色");
    await user.type(screen.getByLabelText("参考文本"), "大家好");
    await user.upload(screen.getByLabelText("参考音频"), new File(["mp3"], "sample.mp3", { type: "audio/mpeg" }));
    await screen.findByText(/音频有效/);
    await user.click(screen.getByRole("button", { name: "上传音色" }));
    await waitFor(() => expect(pair.resourceClient.createVoice).toHaveBeenCalledOnce());
    const uploaded = vi.mocked(pair.resourceClient.createVoice).mock.calls[0][1];
    expect(uploaded.name).toBe("sample.wav");
    expect(uploaded.type).toBe("audio/wav");
    await expect(validateVoiceWav(uploaded)).resolves.toMatchObject({ durationMs: 4000, sampleRate: 48000 });
  });

  it("音频处理中禁止上传，重新选择后旧解码失败不会覆盖新文件", async () => {
    let rejectDecode!: (reason: Error) => void;
    const decode = vi.fn(() => new Promise<never>((_, reject) => { rejectDecode = reject; }));
    vi.stubGlobal("OfflineAudioContext", class { decodeAudioData = decode; });
    const user = userEvent.setup();
    const pair = clients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });
    await user.type(screen.getByLabelText("音色名称"), "新音色");
    await user.type(screen.getByLabelText("参考文本"), "大家好");
    await user.upload(screen.getByLabelText("参考音频"), new File(["mp3"], "old.mp3", { type: "audio/mpeg" }));
    await waitFor(() => expect(decode).toHaveBeenCalledOnce());
    expect(screen.getByText("正在处理音频…")).toBeVisible();
    expect(screen.getByRole("button", { name: "上传音色" })).toBeDisabled();

    const wav = pcm16Wav();
    await user.upload(screen.getByLabelText("参考音频"), wav);
    await screen.findByText(/音频有效/);
    await act(async () => rejectDecode(new Error("旧文件损坏")));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByText("正在处理音频…")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "上传音色" }));
    await waitFor(() => expect(pair.resourceClient.createVoice).toHaveBeenCalledWith(
      expect.anything(), wav, expect.any(AbortSignal),
    ));
  });

  it("初次使用没有默认音色，上传自己的素材后才显示可选音色", async () => {
    const user = userEvent.setup();
    const pair = clients(resourceSnapshot({ voices: [], characters: [], active_voice_id: "", active_character_id: null, default_voice_available: false }));
    pair.resourceClient.createVoice = vi.fn().mockResolvedValue(resourceSnapshot({ characters: [], active_voice_id: "", active_character_id: null, default_voice_available: false }));
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });

    expect(within(screen.getByRole("list", { name: "可用音色" })).queryAllByRole("listitem")).toHaveLength(0);
    expect(screen.queryByText("当前音色")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "训练初始音色" })).toHaveAttribute("href", "#training");
    await user.type(screen.getByLabelText("音色名称"), "我的音色");
    await user.type(screen.getByLabelText("参考文本"), "这是我的参考声音");
    await user.upload(screen.getByLabelText("参考音频"), pcm16Wav());
    await screen.findByText(/音频有效/);
    await user.click(screen.getByRole("button", { name: "上传音色" }));

    expect(await screen.findByRole("button", { name: "设为当前音色" })).toBeEnabled();
    expect(within(screen.getByRole("list", { name: "可用音色" })).getAllByRole("listitem")).toHaveLength(1);
    expect(screen.queryByText("当前音色")).not.toBeInTheDocument();
  });

  it("已有配置的默认音色仍可主动选用", async () => {
    const user = userEvent.setup();
    const pair = clients();
    pair.resourceClient.selectVoice = vi.fn().mockResolvedValue(resourceSnapshot({ active_voice_id: "default" }));
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });
    const row = within(screen.getByRole("list", { name: "可用音色" })).getByText("配置的默认音色").closest("li")!;

    await user.click(within(row).getByRole("button", { name: "设为当前音色" }));

    expect(pair.resourceClient.selectVoice).toHaveBeenCalledWith({ id: "default" }, expect.any(AbortSignal));
    expect(within(row).getByText("当前音色")).toBeVisible();
  });

  it("选择音色后只按服务端成功快照更新当前标记", async () => {
    const user = userEvent.setup();
    const initial = resourceSnapshot({ active_voice_id: "default" });
    const pair = clients(initial);
    pair.resourceClient.selectVoice = vi.fn().mockResolvedValue(resourceSnapshot({ active_voice_id: "voice-1" }));
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });

    const choose = screen.getAllByRole("button", { name: "设为当前音色" }).find((button) => !button.hasAttribute("disabled"));
    expect(choose).toBeDefined();
    await user.click(choose as HTMLButtonElement);

    expect(pair.resourceClient.selectVoice).toHaveBeenCalledWith({ id: "voice-1" }, expect.any(AbortSignal));
    expect(screen.getAllByText("当前音色")).toHaveLength(1);
    expect(within(screen.getByRole("list", { name: "可用音色" })).getByText("温柔旁白").closest("li"))
      .toHaveTextContent("当前音色");
  });

  it("在浏览器内拒绝静音文件，并上传有效 WAV 及对应元数据", async () => {
    const user = userEvent.setup();
    const pair = clients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });

    await user.upload(screen.getByLabelText("参考音频"), pcm16Wav({ amplitude: 0 }));
    expect(await screen.findByRole("alert")).toHaveTextContent("不能是静音");
    expect(pair.resourceClient.createVoice).not.toHaveBeenCalled();

    await user.type(screen.getByLabelText("音色名称"), "清亮声音");
    await user.selectOptions(screen.getByLabelText("参考文本语言"), "zh");
    await user.type(screen.getByLabelText("参考文本"), "大家晚上好");
    const audio = pcm16Wav();
    await user.upload(screen.getByLabelText("参考音频"), audio);
    expect(await screen.findByText(/音频有效：4\.0 秒/)).toBeVisible();
    await user.click(screen.getByRole("button", { name: "上传音色" }));

    await waitFor(() => expect(pair.resourceClient.createVoice).toHaveBeenCalledWith(
      { name: "清亮声音", language: "zh", reference_text: "大家晚上好" }, audio, expect.any(AbortSignal),
    ));
  });

  it("缺失音色文件时展示原因并禁止选择和试听", async () => {
    const pair = clients(resourceSnapshot({
      active_voice_id: "default",
      voices: [voice({ available: false, error: "参考文件缺失，请重新上传" })],
    }));
    render(<ResourcesPanel {...pair} />);

    expect(await screen.findByText("参考文件缺失，请重新上传")).toBeVisible();
    const row = screen.getByText("温柔旁白", { selector: "strong" }).closest("li");
    expect(row).not.toBeNull();
    expect(row?.querySelector<HTMLButtonElement>("button.primary-button")).toBeDisabled();
    expect(Array.from(row?.querySelectorAll("button") ?? []).find((button) => button.textContent === "试听")).toBeDisabled();
  });

  it("试听只报告真实队列状态并链接播报记录", async () => {
    const user = userEvent.setup();
    const pair = clients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "音色管理" });

    await user.click(screen.getByRole("button", { name: "试听" }));

    expect(await screen.findByText(/试听已加入播报队列/)).toBeVisible();
    expect(screen.getByRole("link", { name: "查看播报状态" })).toHaveAttribute("href", "#history-heading");
    expect(pair.speechClient.submitSpeech).toHaveBeenCalledWith(
      { text: voice().reference_text, voice_id: "voice-1" }, expect.any(AbortSignal),
    );
  });
});
