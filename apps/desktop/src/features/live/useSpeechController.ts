import { useEffect, useRef, useState } from "react";
import type { ServerStatus, SpeechRequest, SpeechStatus } from "@meowlive/contracts";
import type { ServerClient } from "../../services/server";
import { useFeedback } from "../../app/feedback/OperationFeedback";

const historyLimit = 50;
const activeStatuses = new Set<SpeechStatus>(["queued", "synthesizing", "ready", "playing"]);

export function isActiveSpeech(status: SpeechStatus): boolean {
  return activeStatuses.has(status);
}

function boundHistory(status: ServerStatus): ServerStatus {
  const recentIds = new Set(status.speeches.filter((task) => !isActiveSpeech(task.status)).slice(-historyLimit).map((task) => task.id));
  return { ...status, speeches: status.speeches.filter((task) => isActiveSpeech(task.status) || recentIds.has(task.id)) };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "请求失败，请稍后重试。";
}

export function useSpeechController(client: ServerClient, pollIntervalMs: number) {
  const feedback = useFeedback();
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<"speech" | "stop" | null>(null);
  const pollController = useRef<AbortController | null>(null);
  const actionController = useRef<AbortController | null>(null);
  const revision = useRef(0);
  const knownTasks = useRef(new Map<string, SpeechStatus>());
  function acceptStatus(next: ServerStatus) {
    for (const task of next.speeches) {
      const key = `${task.generation}:${task.id}`;
      const previous = knownTasks.current.get(key);
      if (previous && isActiveSpeech(previous)) {
        if (task.status === "failed" || task.status === "unknown") feedback.error("播报失败", task.error || "执行端断开，无法确认播放结果。");
        if (task.status === "completed") feedback.success("播报已完成", task.text);
      }
    }
    knownTasks.current = new Map(next.speeches.map(task => [`${task.generation}:${task.id}`, task.status]));
    setStatus(boundHistory(next));
  }

  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    setStatus(null);
    setConnectionError(null);
    setActionError(null);
    setPendingAction(null);
    knownTasks.current.clear();

    async function refresh() {
      if (disposed) return;
      if (!actionController.current) {
        const controller = new AbortController();
        pollController.current = controller;
        const startedRevision = revision.current;
        try {
          const next = await client.getStatus(controller.signal);
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) {
            acceptStatus(next);
            setConnectionError(null);
            feedback.clearIssue("speech:status");
          }
        } catch (error) {
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) { setConnectionError(errorMessage(error)); feedback.reportIssue("speech:status", "播报状态读取失败", error); }
        } finally {
          if (pollController.current === controller) pollController.current = null;
        }
      }
      if (!disposed) timer = setTimeout(() => { void refresh(); }, pollIntervalMs);
    }

    void refresh();
    return () => {
      disposed = true;
      clearTimeout(timer);
      pollController.current?.abort();
      actionController.current?.abort();
      actionController.current = null;
      revision.current += 1;
    };
  }, [client, pollIntervalMs, feedback]);

  async function mutate(kind: "speech" | "stop", request?: SpeechRequest): Promise<boolean> {
    if (actionController.current) return false;
    const controller = new AbortController();
    actionController.current = controller;
    revision.current += 1;
    pollController.current?.abort();
    setPendingAction(kind);
    setActionError(null);
    try {
      if (kind === "speech" && request) {
        const snapshot = await client.submitSpeech(request, controller.signal);
        if (controller.signal.aborted) return false;
        knownTasks.current.set(`${snapshot.generation}:${snapshot.id}`, snapshot.status);
        setStatus((current) => current ? boundHistory({
          ...current,
          speeches: [...current.speeches.filter((entry) => entry.id !== snapshot.id), snapshot],
        }) : current);
        if (snapshot.status === "failed" || snapshot.status === "unknown") { feedback.error("播报失败", snapshot.error || "播报结果未知，请检查执行端。"); return false; }
        feedback.notify({ kind: snapshot.status === "completed" ? "success" : "info", title: snapshot.status === "completed" ? "播报已完成" : "播报已提交", message: snapshot.status === "completed" ? snapshot.text : "任务已接收，请在播报记录查看合成与播放结果。" });
      } else {
        const next = await client.stop(controller.signal);
        if (controller.signal.aborted) return false;
        acceptStatus(next);
        feedback.success("停止播报请求已处理", "已更新播报状态，请查看播报记录。");
      }
      setConnectionError(null);
      return true;
    } catch (error) {
      if (!controller.signal.aborted) { setActionError(errorMessage(error)); feedback.error(kind === "speech" ? "提交播报失败" : "停止播报失败", error); }
      return false;
    } finally {
      if (actionController.current === controller) {
        actionController.current = null;
        setPendingAction(null);
      }
    }
  }

  return {
    status,
    connectionError,
    actionError,
    pendingAction,
    submit: (request: SpeechRequest) => mutate("speech", request),
    stop: () => mutate("stop"),
  };
}
