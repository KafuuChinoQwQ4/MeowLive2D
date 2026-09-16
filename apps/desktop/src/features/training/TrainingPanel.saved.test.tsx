import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModelVersion, TrainingSnapshot } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { resourceSnapshot, voice } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";

function version(overrides: Partial<ModelVersion> = {}): ModelVersion {
  return { id: "version-1", job_id: "version-1", voice_id: "voice-1", name: "温柔训练音色", engine: "gpt-sovits", model_version: "v2", auditioned: true, saved: true, active: false, available: true, created_at_ms: 1, ...overrides };
}
function setup(versions = [version()]) {
  let snapshot: TrainingSnapshot = { enabled: true, busy: false, versions, jobs: versions.map(v => ({ id: v.job_id, name: v.name, voice_id: v.voice_id, status: "completed", progress: 100, message: "训练完成", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: v.id })) };
  let assets = resourceSnapshot({ active_voice_id: "default", voices: [voice(), voice({ id: "voice-2", name: "活泼声音" })] });
  const client: TrainingClient = {
    snapshot: vi.fn(async () => snapshot),
    preset: vi.fn().mockResolvedValue({ mode: "cloud", model: "", local_only: false, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "云端预设" }),
    create: vi.fn(), cancel: vi.fn(), measure: vi.fn(),
    audition: vi.fn(async id => {
      snapshot = { ...snapshot, versions: snapshot.versions.map(v => v.id === id ? { ...v, auditioned: true } : v) };
      return new Blob([new Uint8Array(48)], { type: "audio/wav" });
    }),
    save: vi.fn(async id => {
      snapshot = { ...snapshot, versions: snapshot.versions.map(v => v.id === id ? { ...v, saved: true } : v) };
      return snapshot;
    }),
    activate: vi.fn(async id => {
      const selected = snapshot.versions.find(v => v.id === id)!;
      snapshot = { ...snapshot, versions: snapshot.versions.map(v => v.voice_id === selected.voice_id ? { ...v, active: v.id === id, saved: v.saved || v.id === id } : v) };
      return snapshot;
    }),
  };
  const resources = {
    getSnapshot: vi.fn(async () => assets),
    selectVoice: vi.fn(async ({ id }: { id: string }) => { assets = { ...assets, active_voice_id: id }; return assets; }),
  } as unknown as ResourcesClient;
  return { client, resources };
}

afterEach(() => vi.unstubAllGlobals());

describe("保存的训练音色", () => {
  it("试听确认后保存，重开页面可以直接选用而不必再次试听", async () => {
    const deps = setup([version({ auditioned: false, saved: false })]);
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:preview"), revokeObjectURL: vi.fn() }));
    const user = userEvent.setup();
    const view = render(<TrainingPanel {...deps} />);
    const save = await screen.findByRole("button", { name: "保存音色" });
    expect(save).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "生成版本试听" }));
    await screen.findByLabelText("版本试听音频");
    expect(save).toBeDisabled();
    await user.click(screen.getByLabelText("我已试听并确认此版本效果"));
    await user.click(save);
    await screen.findByRole("button", { name: "音色已保存" });
    expect(deps.resources.selectVoice).not.toHaveBeenCalled();
    view.unmount();
    render(<TrainingPanel {...deps} />);
    const select = await screen.findByLabelText("选择已保存音色");
    await user.selectOptions(select, "version-1");
    await user.click(screen.getByRole("button", { name: "使用所选音色" }));
    await screen.findByText("当前使用：温柔训练音色");
    expect(deps.client.audition).toHaveBeenCalledTimes(1);
    expect(deps.client.activate).toHaveBeenCalledWith("version-1", expect.any(AbortSignal));
    expect(deps.resources.selectVoice).toHaveBeenCalledWith({ id: "voice-1" }, expect.any(AbortSignal));
  });

  it("不同音色及同音色的不同版本都能来回切换", async () => {
    const deps = setup([version(), version({ id: "version-2", job_id: "version-2", voice_id: "voice-2", name: "活泼训练音色" }), version({ id: "version-3", job_id: "version-3", name: "温柔第二版" })]);
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    const select = await screen.findByLabelText("选择已保存音色");
    for (const [id, name, voiceId] of [["version-1", "温柔训练音色", "voice-1"], ["version-2", "活泼训练音色", "voice-2"], ["version-3", "温柔第二版", "voice-1"], ["version-1", "温柔训练音色", "voice-1"]]) {
      await user.selectOptions(select, id);
      await user.click(screen.getByRole("button", { name: "使用所选音色" }));
      await screen.findByText(`当前使用：${name}`);
      expect(deps.resources.selectVoice).toHaveBeenLastCalledWith({ id: voiceId }, expect.any(AbortSignal));
    }
    expect(deps.client.audition).not.toHaveBeenCalled();
  });

  it("保存失败保留确认并允许重试，不把失败显示成已保存", async () => {
    const deps = setup([version({ saved: false })]);
    vi.mocked(deps.client.save).mockRejectedValueOnce(new Error("保存失败：磁盘不可写"));
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:preview"), revokeObjectURL: vi.fn() }));
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    await user.click(await screen.findByRole("button", { name: "生成版本试听" }));
    await user.click(await screen.findByLabelText("我已试听并确认此版本效果"));
    await user.click(screen.getByRole("button", { name: "保存音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("磁盘不可写");
    expect(screen.getByRole("button", { name: "保存音色" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "音色已保存" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "保存音色" }));
    await screen.findByRole("button", { name: "音色已保存" });
  });

  it("模型启用失败时不切参考音色，也不显示切换成功", async () => {
    const deps = setup();
    vi.mocked(deps.client.activate).mockRejectedValueOnce(new Error("模型文件缺失"));
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "version-1");
    await user.click(screen.getByRole("button", { name: "使用所选音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("模型文件缺失");
    expect(deps.resources.selectVoice).not.toHaveBeenCalled();
    expect(screen.queryByText("当前使用：温柔训练音色")).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "使用所选音色" })).toBeEnabled());
  });

  it("缺失的模型和参考音频不能从保存列表中选用", async () => {
    const deps = setup([version({ available: false }), version({ id: "version-2", job_id: "version-2", voice_id: "missing-voice", name: "缺参考音频" })]);
    render(<TrainingPanel {...deps} />);
    const select = await screen.findByLabelText("选择已保存音色");
    expect(within(select).getByRole("option", { name: /温柔训练音色/ })).toBeDisabled();
    expect(within(select).getByRole("option", { name: /缺参考音频/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "使用所选音色" })).toBeDisabled();
  });

  it("参考音色选择失败时显示错误，仍可重试完成切换", async () => {
    const deps = setup();
    vi.mocked(deps.resources.selectVoice).mockRejectedValueOnce(new Error("参考音色选择失败"));
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "version-1");
    await user.click(screen.getByRole("button", { name: "使用所选音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("参考音色选择失败");
    expect(screen.getByText("当前使用：配置的默认音色")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "使用所选音色" }));
    await screen.findByText("当前使用：温柔训练音色");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
