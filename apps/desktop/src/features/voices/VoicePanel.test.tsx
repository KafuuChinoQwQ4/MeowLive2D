import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";
import { pcm16Wav, resourceSnapshot, voice } from "../../test/resource-fixtures";
import { ResourcesPanel } from "../../app/resources";

function clients(snapshot = resourceSnapshot()) {
  const resourceClient: ResourcesClient = {
    baseUrl: "http://localhost:19600",
    getSnapshot: vi.fn().mockResolvedValue(snapshot),
    createVoice: vi.fn().mockResolvedValue(snapshot),
    selectVoice: vi.fn().mockResolvedValue(snapshot),
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
    const row = screen.getByText("温柔旁白").closest("li");
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
