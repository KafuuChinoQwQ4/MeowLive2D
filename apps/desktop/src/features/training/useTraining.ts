import { useCallback, useEffect, useRef, useState } from "react";
import type { ModelVersion, ResourceSnapshot, RuntimePresetSnapshot, TrainingSnapshot, TrainingModelRuntimeSnapshot, TrainingJob } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";
import { ServerRequestError } from "../../services/server/responses";
import { failureFeedback, terminalFeedback, type ActionFeedback, type TrainingFeedback, type TrainingNotice } from "./trainingFeedback";
import { useFeedback } from "../../app/feedback/OperationFeedback";

type Query = "training" | "resources" | "preset" | "models";
const queries: Query[] = ["training", "resources", "preset", "models"];
const emptyErrors = { training: "", resources: "", preset: "", models: "" };

export function useTraining(client: TrainingClient, resources: ResourcesClient) {
  const sharedFeedback = useFeedback();
  const [snapshot, setSnapshot] = useState<TrainingSnapshot | null>(null);
  const [assets, setAssets] = useState<ResourceSnapshot | null>(null);
  const [preset, setPreset] = useState<RuntimePresetSnapshot | null>(null);
  const [modelRuntime, setModelRuntime] = useState<TrainingModelRuntimeSnapshot | null>(null);
  const previousModelState = useRef<string | null>(null);
  const [operationError, setError] = useState("");
  const [queryErrors, setQueryErrors] = useState(emptyErrors);
  const [pending, setPending] = useState(false);
  const [audio, setAudio] = useState<{ version: string; url: string } | null>(null);
  const [failedDelete, setFailedDelete] = useState<string | null>(null);
  const [notices, setNotices] = useState<TrainingNotice[]>([]);
  const [inlineNotice, setInlineNotice] = useState<TrainingNotice | null>(null);
  const noticeId = useRef(0);
  const lastJobs = useRef<Map<string, string> | null>(null);
  const notify = useCallback((feedback: TrainingFeedback, restoreFocus?: HTMLElement) => {
    const active = document.activeElement;
    const notice = { ...feedback, id: ++noticeId.current, restoreFocus: restoreFocus ?? (active instanceof HTMLElement ? active : undefined) };
    if (sharedFeedback.enabled) sharedFeedback.notify(notice);
    else if (notice.kind === "error") setNotices(previous => [...previous, notice]);
    else setInlineNotice(notice);
  }, [sharedFeedback]);
  const reportFailure = useCallback((operation: string, error: unknown, restoreFocus?: HTMLElement) => notify(failureFeedback(operation, error), restoreFocus), [notify]);
  const acceptSnapshot = useCallback((value: TrainingSnapshot) => {
    for (const job of value.jobs) {
      const previous = lastJobs.current?.get(job.id);
      if (previous !== undefined && previous !== job.status) {
        const feedback = terminalFeedback(job);
        if (feedback) notify(feedback);
      }
    }
    lastJobs.current = new Map(value.jobs.map(job => [job.id, job.status]));
    setSnapshot(value);
  }, [notify]);
  const trackJob = (job: TrainingJob) => {
    if (!job?.id) return;
    // A poll started before create completed must not erase the new job's status.
    if (context.current) context.current.revisions.training++;
    lastJobs.current ??= new Map();
    lastJobs.current.set(job.id, job.status);
  };
  const context = useRef<{ ctrl: AbortController; revisions: Record<Query, number>; pending: boolean; modelPending: boolean } | null>(null);
  const blob = useRef<string | null>(null);
  const refreshQuery = useCallback(async (query: Query) => {
    const current = context.current;
    if (!current || current.ctrl.signal.aborted || (query === "models" && current.modelPending)) return;
    const revision = ++current.revisions[query];
    const isCurrent = () => context.current === current && !current.ctrl.signal.aborted && revision === current.revisions[query];
    try {
      if (query === "training") {
        const value = await client.snapshot(current.ctrl.signal);
        if (!isCurrent()) return;
        acceptSnapshot(value);
      } else if (query === "resources") {
        const value = await resources.getSnapshot(current.ctrl.signal);
        if (!isCurrent()) return;
        setAssets(value);
      } else if (query === "models") {
        const value = await client.modelStatus(current.ctrl.signal);
        if (!isCurrent()) return;
        setModelRuntime(value);
        if (value.state === "failed" || (value.state === "unavailable" && ["loaded", "loading", "unloading"].includes(previousModelState.current ?? ""))) sharedFeedback.reportIssue("training-model-state", "语音模型异常", value.message);
        else if (value.state !== "unavailable") sharedFeedback.clearIssue("training-model-state");
        previousModelState.current = value.state;
      } else {
        const value = await client.preset(current.ctrl.signal);
        if (!isCurrent()) return;
        setPreset(value);
      }
      setQueryErrors(errors => ({ ...errors, [query]: "" }));
      sharedFeedback.clearIssue(`training-query-${query}`);
    } catch (e) {
      if (isCurrent()) {
        const message = e instanceof Error ? e.message : "读取训练状态失败";
        if (query === "models") setModelRuntime({ supported: true, state: "unavailable", message });
        else setQueryErrors(errors => ({ ...errors, [query]: message }));
        sharedFeedback.reportIssue(`training-query-${query}`, `${{ training: "训练状态", resources: "音色资源", preset: "离线预设", models: "语音模型状态" }[query]}读取失败`, e);
      }
    }
  }, [client, resources, acceptSnapshot, sharedFeedback]);
  const refresh = useCallback(async () => {
    // Resource and preset latency must not hold training controls pending.
    void refreshQuery("resources");
    void refreshQuery("preset");
    void refreshQuery("models");
    await refreshQuery("training");
  }, [refreshQuery]);
  useEffect(() => {
    const current = { ctrl: new AbortController(), revisions: { training: 0, resources: 0, preset: 0, models: 0 }, pending: false, modelPending: false };
    context.current = current;
    lastJobs.current = null; previousModelState.current = null; setNotices([]); setInlineNotice(null);
    setSnapshot(null); setAssets(null); setPreset(null); setModelRuntime(null); setPending(false); setAudio(null); setFailedDelete(null);
    setQueryErrors(emptyErrors); setError("");
    const timers: Partial<Record<Query, ReturnType<typeof setTimeout>>> = {};
    for (const query of queries) {
      const poll = async () => {
        await refreshQuery(query);
        if (!current.ctrl.signal.aborted) timers[query] = setTimeout(() => { void poll(); }, 2000);
      };
      void poll();
    }
    return () => {
      current.ctrl.abort();
      queries.forEach(query => sharedFeedback.clearIssue(`training-query-${query}`));
      sharedFeedback.clearIssue("training-model-state");
      for (const query of queries) clearTimeout(timers[query]);
      if (blob.current) URL.revokeObjectURL(blob.current);
      blob.current = null;
    };
  }, [refreshQuery, sharedFeedback]);
  async function action(work: (signal: AbortSignal) => Promise<void | TrainingFeedback>, feedback: ActionFeedback = { operation: "操作", success: "操作已完成" }) {
    const current = context.current;
    if (!current || current.pending || current.ctrl.signal.aborted) return;
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    current.pending = true;
    for (const query of queries) current.revisions[query]++;
    setPending(true); setError("");
    try {
      const outcome = await work(current.ctrl.signal);
      if (!current.ctrl.signal.aborted) {
        notify(outcome ?? { kind: "success", title: feedback.successTitle ?? `${feedback.operation}成功`, message: feedback.success, tips: [] }, trigger);
        await refresh();
      }
    }
    catch (e) { if (!current.ctrl.signal.aborted) { setError(e instanceof Error ? e.message : "训练操作失败"); reportFailure(feedback.operation, e, trigger); } }
    finally { current.pending = false; if (!current.ctrl.signal.aborted) setPending(false); }
  }
  async function setModelsEnabled(enabled: boolean) {
    await action(async signal => {
      const current = context.current!;
      const previous = modelRuntime;
      current.modelPending = true;
      setModelRuntime({ supported: true, state: enabled ? "loading" : "unloading", message: enabled ? "正在加载语音模型，请稍候…" : "正在释放语音模型内存…" });
      try {
        const value = await client.setModelsEnabled(enabled, signal);
        if (signal.aborted) return;
        current.revisions.models++;
        setModelRuntime(value);
        previousModelState.current = value.state;
      } catch (error) {
        if (!signal.aborted) setModelRuntime(previous);
        throw error;
      } finally {
        current.modelPending = false;
        if (!signal.aborted) void refreshQuery("models");
      }
    }, { operation: enabled ? "启用模型" : "关闭模型", success: enabled ? "语音模型已启用，可以播报或试听。" : "语音模型已关闭，训练文件与音色选择仍保留。" });
  }
  async function audition(version: string, text: string) {
    await action(async signal => {
      const value = await client.audition(version, text, signal);
      if (signal.aborted) return;
      if (blob.current) URL.revokeObjectURL(blob.current);
      blob.current = URL.createObjectURL(value); setAudio({ version, url: blob.current });
    }, { operation: "生成试听", success: "试听音频已生成，请播放并确认效果。" });
  }
  async function saveVersion(id: string) {
    await action(async signal => {
      const value = await client.save(id, signal);
      if (signal.aborted) return;
      context.current!.revisions.training++;
      acceptSnapshot(value);
    }, { operation: "保存音色", success: "训练音色已保存，可以选择使用。" });
  }
  async function deleteVersion(id: string) {
    await action(async signal => {
      let value: TrainingSnapshot;
      try { value = await client.delete(id, signal); }
      catch (error) {
        if (!signal.aborted) setFailedDelete(id);
        throw error;
      }
      if (signal.aborted) return;
      setFailedDelete(null);
      context.current!.revisions.training++;
      acceptSnapshot(value);
      if (audio?.version === id) {
        if (blob.current) URL.revokeObjectURL(blob.current);
        blob.current = null;
        setAudio(null);
      }
    }, { operation: "删除训练记录", success: "所选训练记录及其版本已删除。" });
  }
  async function activateVersion(version: ModelVersion) {
    await action(async signal => {
      const value = await client.activate(version.id, signal);
      if (signal.aborted) return;
      context.current!.revisions.training++;
      acceptSnapshot(value);
      const selected = await resources.selectVoice({ id: version.voice_id }, signal);
      if (signal.aborted) return;
      context.current!.revisions.resources++;
      setAssets(selected);
    }, { operation: "选用音色", success: "已选用此音色；语音模型启用后，后续播报将使用该版本。" });
  }
  async function measure(voice: string, text: string) {
    await action(async signal => {
      const value = await client.measure(voice, text, signal);
      if (signal.aborted) return;
      setPreset(value);
      if (!value.verified) throw new ServerRequestError("measurement_unverified", value.message);
      return { kind: "success", title: "离线测量成功", message: value.message, tips: ["本次结果已显示在离线设置中；更换模型或运行环境后可重新测量。"] };
    }, { operation: "离线测量", success: "本次离线测量通过。" });
  }
  const queryError = queries.map(query => queryErrors[query]).filter(Boolean).join("；");
  const error = [operationError, ...queries.map(query => queryErrors[query])].filter(Boolean).join("；");
  return { snapshot, voices: assets?.voices ?? [], activeVoiceId: assets?.active_voice_id, defaultVoiceAvailable: assets?.default_voice_available ?? false, preset, modelRuntime, setModelsEnabled, error, queryError, setError, pending, audio, failedDelete, action, audition, saveVersion, deleteVersion, activateVersion, measure, refresh, trackJob, notify, reportFailure, notice: notices[0] ?? inlineNotice, dismissNotice: () => setNotices(previous => previous.slice(1)) };
}
