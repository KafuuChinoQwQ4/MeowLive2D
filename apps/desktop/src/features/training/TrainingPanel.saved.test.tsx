import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ModelVersion, TrainingSnapshot } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";
import { ServerRequestError } from "../../services/server/responses";
import type { ResourcesClient } from "../../services/server/resources";
import { resourceSnapshot, voice } from "../../test/resource-fixtures";
import { TrainingPanel } from "./TrainingPanel";

function version(overrides: Partial<ModelVersion> = {}): ModelVersion {
  return { id: "version-1", job_id: "version-1", voice_id: "voice-1", name: "温柔训练音色", engine: "gpt-sovits", model_version: "v2", auditioned: true, saved: true, active: false, available: true, created_at_ms: 1, ...overrides };
}
function setup(versions = [version()]) {
  let snapshot: TrainingSnapshot = { enabled: true, busy: false, versions, jobs: versions.map(v => ({ id: v.job_id, name: v.name, voice_id: v.voice_id, status: "completed", progress: 100, message: "训练完成", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: v.id, performance: { batch_size: 1, data_workers: 1, cpu_threads: 2, gpu_index: 0, low_memory: true } })) };
  let assets = resourceSnapshot({ active_voice_id: "default", voices: [voice(), voice({ id: "voice-2", name: "活泼声音" })] });
  const client: TrainingClient = { modelStatus: vi.fn().mockResolvedValue({ supported: true, state: "loaded", message: "模型已启用" }), setModelsEnabled: vi.fn(),
    snapshot: vi.fn(async () => snapshot),
    preset: vi.fn().mockResolvedValue({ mode: "cloud", model: "", local_only: false, verified: false, max_tokens: 512, timeout_seconds: 30, measurement: null, message: "云端预设" }),
    transcribe: vi.fn(), create: vi.fn(), cancel: vi.fn(), measure: vi.fn(),
    delete: vi.fn(async id => {
      snapshot = { ...snapshot, jobs: snapshot.jobs.filter(j => j.id !== id), versions: snapshot.versions.filter(v => v.id !== id) };
      return snapshot;
    }),
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
  it("下拉框只暂存选择，点击确认后才切换当前音色", async () => {
    const deps = setup([version(), version({ id: "version-2", job_id: "version-2", name: "温柔第二版" })]);
    const user = userEvent.setup();
    const onResourcesChanged = vi.fn();
    render(<TrainingPanel {...deps} onResourcesChanged={onResourcesChanged} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));

    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-2");

    expect(deps.client.activate).not.toHaveBeenCalled();
    expect(deps.resources.selectVoice).not.toHaveBeenCalled();
    expect(screen.getByText("待确认：温柔旁白 · 温柔第二版")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "确认所选音色" }));
    await screen.findByText("当前使用：温柔旁白 · 温柔第二版");
    expect(deps.client.activate).toHaveBeenCalledWith("version-2", expect.any(AbortSignal));
    expect(deps.resources.selectVoice).toHaveBeenCalledWith({ id: "voice-1" }, expect.any(AbortSignal));
    expect(onResourcesChanged).toHaveBeenCalledOnce();
  });

  it("试听确认后保存，重开页面可以直接选用而不必再次试听", async () => {
    const deps = setup([version({ auditioned: false, saved: false })]);
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:preview"), revokeObjectURL: vi.fn() }));
    const user = userEvent.setup();
    const view = render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    const save = await screen.findByRole("button", { name: "保存音色" });
    expect(save).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "生成版本试听" }));
    await screen.findByLabelText("版本试听音频");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(save).toBeDisabled();
    await user.click(screen.getByLabelText("我已试听并确认此版本效果"));
    await user.click(save);
    await screen.findByRole("button", { name: "音色已保存" });
    expect(deps.resources.selectVoice).not.toHaveBeenCalled();
    view.unmount();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    const select = await screen.findByLabelText("选择已保存音色");
    await user.selectOptions(select, "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-1");
    await user.click(screen.getByRole("button", { name: "确认所选音色" }));
    await screen.findByText("当前使用：温柔旁白 · 温柔训练音色");
    expect(deps.client.audition).toHaveBeenCalledTimes(1);
    expect(deps.client.activate).toHaveBeenCalledWith("version-1", expect.any(AbortSignal));
    expect(deps.resources.selectVoice).toHaveBeenCalledWith({ id: "voice-1" }, expect.any(AbortSignal));
  });

  it("不同音色及同音色的不同版本都能来回切换", async () => {
    const deps = setup([version(), version({ id: "version-2", job_id: "version-2", voice_id: "voice-2", name: "活泼训练音色" }), version({ id: "version-3", job_id: "version-3", name: "温柔第二版" })]);
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    const select = await screen.findByLabelText("选择已保存音色");
    expect(within(select).getAllByRole("option")).toHaveLength(3);
    expect(within(screen.getByRole("list", { name: "已训练音色" })).getAllByRole("listitem")).toHaveLength(2);
    for (const [id, name, voiceId] of [["version-1", "温柔训练音色", "voice-1"], ["version-2", "活泼训练音色", "voice-2"], ["version-3", "温柔第二版", "voice-1"], ["version-1", "温柔训练音色", "voice-1"]]) {
      await user.selectOptions(select, voiceId);
      await user.selectOptions(screen.getByLabelText("选择已保存版本"), id);
      await user.click(screen.getByRole("button", { name: "确认所选音色" }));
      await screen.findByText(`当前使用：${voiceId === "voice-1" ? "温柔旁白" : "活泼声音"} · ${name}`);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(deps.resources.selectVoice).toHaveBeenLastCalledWith({ id: voiceId }, expect.any(AbortSignal));
    }
    expect(deps.client.audition).not.toHaveBeenCalled();
  });

  it("保存失败保留确认并允许重试，不把失败显示成已保存", async () => {
    const deps = setup([version({ saved: false })]);
    vi.mocked(deps.client.save).mockRejectedValueOnce(new Error("保存失败：磁盘不可写"));
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:preview"), revokeObjectURL: vi.fn() }));
    const user = userEvent.setup(); render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.click(await screen.findByRole("button", { name: "生成版本试听" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(await screen.findByLabelText("我已试听并确认此版本效果"));
    await user.click(screen.getByRole("button", { name: "保存音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("磁盘不可写");
    expect(screen.getByRole("button", { name: "保存音色" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "音色已保存" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "知道了" }));
    await user.click(screen.getByRole("button", { name: "保存音色" }));
    await screen.findByRole("button", { name: "音色已保存" });
  });

  it("模型启用失败时不切参考音色，也不显示切换成功", async () => {
    const deps = setup();
    vi.mocked(deps.client.activate).mockRejectedValueOnce(new Error("模型文件缺失"));
    const onResourcesChanged = vi.fn();
    const user = userEvent.setup(); render(<TrainingPanel {...deps} onResourcesChanged={onResourcesChanged} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-1");
    await user.click(screen.getByRole("button", { name: "确认所选音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("模型文件缺失");
    expect(deps.resources.selectVoice).not.toHaveBeenCalled();
    expect(onResourcesChanged).not.toHaveBeenCalled();
    expect(screen.queryByText("当前使用：温柔旁白 · 温柔训练音色")).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "确认所选音色" })).toBeEnabled());
  });

  it("缺失的模型和参考音频不能从保存列表中选用", async () => {
    const deps = setup([version({ available: false }), version({ id: "version-2", job_id: "version-2", voice_id: "missing-voice", name: "缺参考音频" })]);
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    const select = await screen.findByLabelText("选择已保存音色");
    expect(within(select).getByRole("option", { name: /温柔旁白/ })).toBeDisabled();
    expect(within(select).getByRole("option", { name: /参考音色缺失/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: "确认所选音色" })).toBeDisabled();
  });

  it("参考音色选择失败时显示错误，仍可重试完成切换", async () => {
    const deps = setup();
    vi.mocked(deps.resources.selectVoice).mockRejectedValueOnce(new Error("参考音色选择失败"));
    const onResourcesChanged = vi.fn();
    const user = userEvent.setup(); render(<TrainingPanel {...deps} onResourcesChanged={onResourcesChanged} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-1");
    await user.click(screen.getByRole("button", { name: "确认所选音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("参考音色选择失败");
    expect(onResourcesChanged).not.toHaveBeenCalled();
    expect(screen.getByText("当前使用：尚未选用训练音色")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "知道了" }));
    await user.click(screen.getByRole("button", { name: "确认所选音色" }));
    await screen.findByText("当前使用：温柔旁白 · 温柔训练音色");
    expect(onResourcesChanged).toHaveBeenCalledOnce();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});


describe("删除训练音色", () => {
  it("确认中显示名称，取消时保留保存的版本", async () => {
    const deps = setup();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    const user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-1");
    await user.click(screen.getByRole("button", { name: "删除所选版本" }));
    expect(confirm).toHaveBeenCalledWith(expect.stringContaining("温柔训练音色"));
    expect(deps.client.delete).not.toHaveBeenCalled();
    expect(screen.getByLabelText("选择已保存音色")).toHaveValue("voice-1");
    expect(screen.getByRole("button", { name: "确认所选音色" })).toBeEnabled();
  });

  it("删除一个版本后音色分组与其他版本保留，试听地址释放", async () => {
    const deps = setup([version(), version({ id: "version-2", job_id: "version-2", name: "保留的音色" })]);
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const revoke = vi.fn();
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:deleted"), revokeObjectURL: revoke }));
    const user = userEvent.setup();
    const view = render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    const select = await screen.findByLabelText("选择已保存音色");
    await user.selectOptions(select, "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-1");
    await user.click(screen.getByText("查看训练版本（2）"));
    await user.selectOptions(screen.getByLabelText("温柔旁白 的训练版本"), "version-1");
    await user.click(screen.getByRole("button", { name: "生成版本试听" }));
    await screen.findByLabelText("版本试听音频");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "删除所选版本" }));
    await waitFor(() => expect(screen.queryByRole("option", { name: /温柔训练音色/ })).not.toBeInTheDocument());
    expect(select).toHaveValue("voice-1");
    expect(screen.getByLabelText("选择已保存版本")).toHaveValue("version-2");
    expect(screen.queryByLabelText("版本试听音频")).not.toBeInTheDocument();
    expect(revoke).toHaveBeenCalledWith("blob:deleted");
    expect(deps.client.delete).toHaveBeenCalledWith("version-1", expect.any(AbortSignal));
    expect(screen.getByRole("button", { name: "删除所选版本" })).toBeEnabled();
    view.unmount();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "voice-1");
    expect(within(screen.getByLabelText("选择已保存版本")).getByRole("option", { name: /保留的音色/ })).toBeVisible();
    expect(screen.queryByRole("option", { name: /温柔训练音色/ })).not.toBeInTheDocument();
  });

  it("删除失败显示服务端错误，保留选择并允许重试", async () => {
    const deps = setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    vi.mocked(deps.client.delete).mockRejectedValueOnce(new Error("删除失败：磁盘不可写"));
    const user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.selectOptions(await screen.findByLabelText("选择已保存音色"), "voice-1");
    await user.selectOptions(screen.getByLabelText("选择已保存版本"), "version-1");
    await user.click(screen.getByRole("button", { name: "删除所选版本" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("磁盘不可写");
    expect(screen.getByLabelText("选择已保存音色")).toHaveValue("voice-1");
    expect(screen.getByRole("button", { name: "删除所选版本" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "知道了" }));
    await user.click(screen.getByRole("button", { name: "删除所选版本" }));
    await waitFor(() => expect(screen.queryByRole("option", { name: /温柔训练音色/ })).not.toBeInTheDocument());
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("记录已删除但文件清理失败后，列表刷新也保留重试入口", async () => {
    const deps = setup();
    const remove = deps.client.delete;
    vi.mocked(deps.client.delete).mockImplementationOnce(async () => {
      vi.mocked(deps.client.snapshot).mockResolvedValue({ enabled: true, busy: false, jobs: [], versions: [] });
      throw new Error("记录已删除，但文件清理失败，请重试删除");
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.click(await screen.findByRole("button", { name: "删除版本 温柔训练音色" }));
    await screen.findByRole("alert");
    await user.click(screen.getByRole("button", { name: "知道了" }));
    await waitFor(() => expect(screen.queryByRole("option", { name: /温柔训练音色/ })).not.toBeInTheDocument(), { timeout: 3000 });
    await user.click(screen.getByRole("button", { name: "重试删除" }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(remove).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("button", { name: "重试删除" })).not.toBeInTheDocument();
  });

  it("资源选择保存失败时保留版本并允许重试删除", async () => {
    const deps = setup();
    vi.mocked(deps.client.delete).mockRejectedValueOnce(new ServerRequestError("training_selection_clear_failed", "取消当前音色选择失败，训练版本未删除，请检查资源存储后重试删除", 500));
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    await user.click(await screen.findByRole("button", { name: "删除版本 温柔训练音色" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("训练版本未删除");
    expect(screen.getByRole("button", { name: "重试删除" })).toBeEnabled();
  });

  it("失败或取消的训练任务也可以确认删除", async () => {
    const deps = setup([]);
    const empty: TrainingSnapshot = { enabled: true, busy: false, jobs: [], versions: [] };
    vi.mocked(deps.client.snapshot).mockResolvedValue({ ...empty, jobs: [{ id: "failed-job", name: "失败的训练", voice_id: "voice-1", status: "failed", progress: 0, message: "训练失败", clip_count: 2, created_at_ms: 1, updated_at_ms: 1, version_id: null, performance: { batch_size: 1, data_workers: 1, cpu_threads: 2, gpu_index: 0, low_memory: true } }] });
    vi.mocked(deps.client.delete).mockImplementation(async () => { vi.mocked(deps.client.snapshot).mockResolvedValue(empty); return empty; });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    fireEvent.click(screen.getByRole("tab", { name: /训练记录/ }));
    await user.click(await screen.findByRole("button", { name: "删除任务 失败的训练" }));
    await waitFor(() => expect(screen.queryByText("失败的训练")).not.toBeInTheDocument());
    expect(deps.client.delete).toHaveBeenCalledWith("failed-job", expect.any(AbortSignal));
  });

  it("忙碌期间不能删除，损坏的模型仍有独立删除入口", async () => {
    const deps = setup([version({ available: false })]);
    const pending = new Promise<TrainingSnapshot>(() => {});
    vi.mocked(deps.client.delete).mockReturnValueOnce(pending);
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const user = userEvent.setup();
    render(<TrainingPanel {...deps} />);
    fireEvent.click(screen.getByRole("tab", { name: /已训练音色/ }));
    const remove = await screen.findByRole("button", { name: "删除版本 温柔训练音色" });
    expect(remove).toBeEnabled();
    await user.click(remove);
    expect(remove).toBeDisabled();
    expect(screen.getByRole("button", { name: "删除所选版本" })).toBeDisabled();
    expect(deps.client.delete).toHaveBeenCalledTimes(1);
  });
});
