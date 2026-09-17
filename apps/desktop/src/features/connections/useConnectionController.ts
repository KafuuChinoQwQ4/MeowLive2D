import { useEffect, useRef, useState } from "react";
import type { LiveConnectionSnapshot } from "@meowlive/contracts";
import type { LiveClient } from "../../services/server/live";
import { useFeedback } from "../../app/feedback/OperationFeedback";

type Action = "connect" | "disconnect";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "请求失败，请稍后重试。";
}

export function useConnectionController(client: LiveClient, pollIntervalMs: number) {
  const feedback = useFeedback();
  const [status, setStatus] = useState<LiveConnectionSnapshot | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<Action | null>(null);
  const pollController = useRef<AbortController | null>(null);
  const actionController = useRef<AbortController | null>(null);
  const revision = useRef(0);
  const mounted = useRef(false);
  const previousPhase = useRef<LiveConnectionSnapshot["phase"] | null>(null);
  useEffect(() => {
    if (status?.last_error || status?.phase === "failed") feedback.reportIssue("live:connection", "直播连接失败", status.last_error || "直播连接失败，请检查平台和房间配置。");
    else if (status) feedback.clearIssue("live:connection");
    if (status && previousPhase.current) {
      if (["connecting", "reconnecting"].includes(previousPhase.current) && status.phase === "connected") feedback.success("直播间已连接", "已连接直播平台，事件将自动进入处理队列。");
      if (previousPhase.current === "disconnecting" && status.phase === "disconnected") feedback.success("直播间已断开", "已停止接收直播平台事件。");
    }
    previousPhase.current = status?.phase ?? null;
  }, [status, feedback]);

  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    mounted.current = true;
    setStatus(null);
    setConnectionError(null);
    setActionError(null);
    setPendingAction(null);

    async function refresh() {
      if (disposed) return;
      if (!actionController.current) {
        const controller = new AbortController();
        pollController.current = controller;
        const startedRevision = revision.current;
        try {
          const next = await client.getStatus(controller.signal);
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) {
            setStatus(next);
            setConnectionError(null);
            feedback.clearIssue("live:status");
          }
        } catch (error) {
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) {
            setConnectionError(errorMessage(error));
            feedback.reportIssue("live:status", "直播状态读取失败", error);
          }
        } finally {
          if (pollController.current === controller) pollController.current = null;
        }
      }
      if (!disposed) timer = setTimeout(() => { void refresh(); }, pollIntervalMs);
    }

    void refresh();
    return () => {
      disposed = true;
      mounted.current = false;
      clearTimeout(timer);
      pollController.current?.abort();
      actionController.current?.abort();
      actionController.current = null;
      revision.current += 1;
    };
  }, [client, pollIntervalMs, feedback]);

  async function mutate(kind: Action): Promise<boolean> {
    if (actionController.current) return false;
    const controller = new AbortController();
    actionController.current = controller;
    revision.current += 1;
    pollController.current?.abort();
    setPendingAction(kind);
    setActionError(null);
    try {
      const next = await client[kind](controller.signal);
      if (controller.signal.aborted || !mounted.current) return false;
      setStatus(next);
      setConnectionError(null);
      if (next.phase === "failed" || next.last_error) {
        feedback.clearIssue("live:connection");
        feedback.reportIssue("live:connection", "直播连接失败", next.last_error || "连接未成功，请检查直播配置。");
        return false;
      }
      feedback.notify({ kind: next.phase === "connected" || next.phase === "disconnected" ? "success" : "info",
        title: kind === "connect" ? (next.phase === "connected" ? "直播间已连接" : "直播连接请求已提交") : (next.phase === "disconnected" ? "直播间已断开" : "断开请求已提交"),
        message: "请在直播连接面板查看当前状态。" });
      return true;
    } catch (error) {
      if (!controller.signal.aborted && mounted.current) { setActionError(errorMessage(error)); feedback.error(kind === "connect" ? "连接直播间失败" : "断开直播间失败", error); }
      return false;
    } finally {
      if (actionController.current === controller) {
        actionController.current = null;
        if (mounted.current) setPendingAction(null);
      }
    }
  }

  return {
    status,
    connectionError,
    actionError,
    pendingAction,
    connect: () => mutate("connect"),
    disconnect: () => mutate("disconnect"),
  };
}
