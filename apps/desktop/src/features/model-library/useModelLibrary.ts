import { useEffect, useRef, useState } from "react";
import type { ModelLibrarySnapshot } from "@meowlive/contracts";
import type { ModelLibraryClient } from "../../services/model-library";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export function useModelLibrary(client: ModelLibraryClient, token: string | null, onSelected: () => void) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<ModelLibrarySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stale, setStale] = useState(true);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const action = useRef<AbortController | null>(null);
  const polling = useRef<AbortController | null>(null);
  const sequence = useRef(0);
  const mounted = useRef(false);
  const refreshRequested = useRef(false);
  const downloads = useRef(new Map<string, string>());
  useEffect(() => {
    for (const download of snapshot?.downloads ?? []) {
      const key = `models:download:${download.id}`;
      const previous = downloads.current.get(download.id);
      if (download.state === "failed") feedback.reportIssue(key, "模型下载失败", download.message);
      else feedback.clearIssue(key);
      if (previous && ["queued", "downloading"].includes(previous) && download.state === "completed") feedback.success("模型下载完成", download.message || "下载完成，请扫描并选择可用模型。");
    }
    downloads.current = new Map(snapshot?.downloads.map(item => [item.id, item.state]) ?? []);
  }, [snapshot, feedback]);
  useEffect(() => {
    mounted.current = true;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function poll() {
      if (!action.current) {
        const generation = sequence.current;
        const controller = new AbortController();
        polling.current = controller;
        try {
          const next = await client.getStatus(controller.signal);
          if (!cancelled && !controller.signal.aborted && generation === sequence.current) {
            setSnapshot(next); setError(null); setStale(false); feedback.clearIssue("models:status");
            if (refreshRequested.current) { refreshRequested.current = false; feedback.success("模型状态已刷新", "已读取最新模型和下载状态。"); }
          }
        } catch (failure) {
          if (!cancelled && !controller.signal.aborted && generation === sequence.current) {
            setError(failure instanceof Error ? failure.message : "模型状态读取失败"); setStale(true);
            if (refreshRequested.current) { refreshRequested.current = false; feedback.error("刷新模型状态失败", failure); }
            else feedback.reportIssue("models:status", "模型状态读取失败", failure);
          }
        } finally { if (polling.current === controller) polling.current = null; }
      }
      if (!cancelled) timer = setTimeout(() => void poll(), 2500);
    }
    void poll();
    return () => { cancelled = true; mounted.current = false; clearTimeout(timer); polling.current?.abort(); action.current?.abort(); action.current = null; };
  }, [client, attempt, feedback]);
  async function run(kind: "scan" | "select" | "download" | "cancel", id?: string) {
    if (!token || action.current || stale || !snapshot?.environment.ready) return;
    const controller = new AbortController();
    action.current = controller; sequence.current += 1; polling.current?.abort(); setBusy(true); setError(null);
    try {
      const next = kind === "scan" ? await client.scan(token, controller.signal) : await client[kind](id!, token, controller.signal);
      if (mounted.current && !controller.signal.aborted) {
        setSnapshot(next); setStale(false); if (kind === "select") onSelected();
        const download = kind === "download" ? next.downloads.find(item => item.model_id === id) : null;
        if (download?.state === "failed") {
          feedback.clearIssue(`models:download:${download.id}`);
          feedback.reportIssue(`models:download:${download.id}`, "模型下载失败", download.message);
        } else feedback.notify({ kind: kind === "download" || kind === "cancel" ? "info" : "success",
          title: ({ scan: "模型扫描完成", select: "模型已选择", download: "下载请求已提交", cancel: "取消下载请求已提交" })[kind],
          message: kind === "select" && next.installed.find(item => item.id === id)?.purpose === "asr" ? "语音识别模型已选择，下一次提取文本或仅语音训练时生效。" : kind === "download" ? "请在下载列表查看进度和最终结果；模型可用性以扫描结果为准。" : kind === "select" ? "模型选择已更新，请查看运行环境状态确认是否需要重启 TTS。" : kind === "scan" ? `已扫描模型目录，发现 ${next.installed.length} 个模型。` : "请查看下载列表确认最新状态。" });
      }
    } catch (failure) {
      if (mounted.current && !controller.signal.aborted) { setError(`${failure instanceof Error ? failure.message : "模型操作失败"} 请刷新状态确认结果。`); setStale(true); feedback.error("模型操作失败", failure); }
    } finally { if (action.current === controller) action.current = null; if (mounted.current && !controller.signal.aborted) setBusy(false); }
  }
  return { snapshot, error, stale, busy, run, refresh: () => { refreshRequested.current = true; setAttempt(value => value + 1); } };
}
