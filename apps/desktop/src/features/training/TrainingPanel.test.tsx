import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { resourceSnapshot, pcm16Wav, voice } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";
const preset = { mode: "local", model: "small", local_only: true, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "本地预设未验证" };
function setup() {
  const client: TrainingClient = { snapshot: vi.fn().mockResolvedValue({ enabled: true, busy: false, jobs: [], versions: [] }),
    create: vi.fn().mockResolvedValue({}), cancel: vi.fn(), save: vi.fn(), activate: vi.fn(), audition: vi.fn(), preset: vi.fn().mockResolvedValue(preset), measure: vi.fn() };
  const resources = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()), selectVoice: vi.fn().mockResolvedValue(resourceSnapshot()) } as unknown as ResourcesClient;
  return { client, resources };
}
describe("训练面板", () => {
  it.each(["", "default"])("无初始音色时引导上传素材，兼容空选择与旧默认标识 %s", async activeVoiceId => {
    const deps = setup();
    deps.resources.getSnapshot = vi.fn().mockResolvedValue(resourceSnapshot({ voices: [], characters: [], active_voice_id: activeVoiceId, active_character_id: null, default_voice_available: false }));
    render(<TrainingPanel {...deps} />);

    expect(await screen.findByText("当前使用：未设置音色")).toBeVisible();
    expect(screen.getByRole("link", { name: "上传参考声音" })).toHaveAttribute("href", "#resources");
    expect(screen.getByLabelText("训练音色")).toHaveValue("");
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "使用所选音色" })).toBeDisabled();
  });

  it("只有两个片段文本已核对后才提交训练", async () => {
    const user = userEvent.setup(); const deps = setup(); render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    await user.type(screen.getByLabelText("训练名称"), "新音色版本");
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    await user.upload(screen.getByLabelText("训练片段"), [pcm16Wav(), pcm16Wav()]);
    await user.type(screen.getByLabelText("片段 1 文本"), "第一段");
    await user.type(screen.getByLabelText("片段 2 文本"), "第二段");
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
    await user.click(screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致"));
    await user.click(screen.getByRole("button", { name: "开始训练" }));
    await waitFor(() => expect(deps.client.create).toHaveBeenCalledWith(expect.objectContaining({ reviewed: true, voice_id: "voice-1", gpt_epochs: 1 }), expect.any(Array), expect.any(AbortSignal)));
    expect(screen.getByText("本地预设未验证")).toBeVisible();
  });
  it("取消训练不依赖播放器且卸载会取消状态请求", async () => {
    const deps = setup(); deps.client.snapshot = vi.fn().mockResolvedValue({ enabled: true, busy: true, jobs: [{ id: "job", name: "正在训练", status: "training", progress: 40, message: "训练中", clip_count: 2, created_at_ms: 1 }], versions: [] });
    deps.client.cancel = vi.fn().mockResolvedValue({}); const user = userEvent.setup();
    const view = render(<TrainingPanel {...deps} />); await screen.findByRole("button", { name: "取消训练" });
    await user.click(screen.getByRole("button", { name: "取消训练" }));
    expect(deps.client.cancel).toHaveBeenCalledWith("job", expect.any(AbortSignal));
    const signal = vi.mocked(deps.client.snapshot).mock.calls[0]?.[0]; view.unmount(); expect(signal?.aborted).toBe(true);
  });
  it("生成试听后仍需听取确认，卸载释放音频地址", async () => {
    const deps = setup();
    const version = { id: "version", job_id: "version", voice_id: "voice-1", name: "成对权重", engine: "gpt-sovits", model_version: "v2", saved: false, active: false, available: true, auditioned: true, created_at_ms: 1 };
    const snapshot = { enabled: true, busy: false, jobs: [], versions: [version] };
    deps.client.snapshot = vi.fn().mockResolvedValue(snapshot);
    deps.client.audition = vi.fn().mockResolvedValue(new Blob([new Uint8Array(48)], { type: "audio/wav" }));
    deps.client.activate = vi.fn().mockResolvedValue(snapshot);
    const create = vi.fn().mockReturnValue("blob:audition"), revoke = vi.fn();
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: create, revokeObjectURL: revoke }));
    const user = userEvent.setup(); const view = render(<TrainingPanel {...deps} />);
    await screen.findByText("成对权重"); expect(screen.getByRole("button", { name: "启用此版本" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "生成版本试听" }));
    await screen.findByLabelText("版本试听音频");
    await user.click(screen.getByLabelText("我已试听并确认此版本效果"));
    await user.click(screen.getByRole("button", { name: "启用此版本" }));
    expect(deps.client.activate).toHaveBeenCalledWith("version", expect.any(AbortSignal));
    view.unmount(); expect(revoke).toHaveBeenCalledWith("blob:audition"); vi.unstubAllGlobals();
  });
});


describe("素材审核目标", () => {
  it("变更音色撤销确认且不可用音色不能提交", async () => {
    const deps = setup();
    deps.resources.getSnapshot = vi.fn().mockResolvedValue(resourceSnapshot({ voices: [voice(), voice({ id: "voice-2", name: "另一音色" })] }));
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    await screen.findByText("训练就绪");
    await user.type(screen.getByLabelText("训练名称"), "版本");
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-1");
    await user.upload(screen.getByLabelText("训练片段"), [pcm16Wav(), pcm16Wav()]);
    await user.type(screen.getByLabelText("片段 1 文本"), "第一段");
    await user.type(screen.getByLabelText("片段 2 文本"), "第二段");
    const reviewed = screen.getByLabelText("我已逐片听取并核对文本，素材与所选音色一致");
    await user.click(reviewed);
    await user.selectOptions(screen.getByLabelText("训练音色"), "voice-2");
    expect(reviewed).not.toBeChecked();
    expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled();
    await user.click(reviewed);
    expect(screen.getByRole("button", { name: "开始训练" })).toBeEnabled();
    vi.mocked(deps.resources.getSnapshot).mockResolvedValue(resourceSnapshot({ voices: [voice(), voice({ id: "voice-2", available: false })] }));
    await waitFor(() => expect(screen.getByRole("button", { name: "开始训练" })).toBeDisabled(), { timeout: 3000 });
    expect(screen.getByRole("button", { name: "测量本地 LLM 与 TTS" })).toBeDisabled();
    expect(reviewed).not.toBeChecked();
  });
  it("预设失败时仍提供运行中任务的取消入口", async () => {
    const deps = setup();
    deps.client.snapshot = vi.fn().mockResolvedValue({ enabled: true, busy: true, jobs: [{ id: "job", name: "运行任务", status: "training", progress: 40, message: "训练中", clip_count: 2, created_at_ms: 1 }], versions: [] });
    deps.client.preset = vi.fn().mockRejectedValue(new Error("预设不可用"));
    render(<TrainingPanel {...deps} />);
    expect(await screen.findByRole("button", { name: "取消训练" })).toBeEnabled();
    expect(screen.getByRole("alert")).toHaveTextContent("预设不可用");
  });
});
