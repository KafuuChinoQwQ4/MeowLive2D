import { useEffect, useRef, useState } from "react";
import type { AgentSettings, AgentSnapshot, EventBatchRequest, EventBatchResult } from "@meowlive/contracts";
import type { AgentClient } from "../../services/server/agent";
import { useFeedback } from "../../app/feedback/OperationFeedback";

type Action = "settings" | "pause" | "resume" | "events";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "请求失败，请稍后重试。";
}

export function useAgentController(client: AgentClient, pollIntervalMs: number) {
  const feedback = useFeedback();
  const [status, setStatus] = useState<AgentSnapshot | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<Action | null>(null);
  const pollController = useRef<AbortController | null>(null);
  const actionController = useRef<AbortController | null>(null);
  const revision = useRef(0);
  const eventStates = useRef(new Map<string, string>());
  useEffect(() => {
    if (status?.last_error) feedback.reportIssue("agent:runtime", "Agent 运行失败", status.last_error);
    else if (status) feedback.clearIssue("agent:runtime");
    for (const entry of status?.events ?? []) {
      const previous = eventStates.current.get(entry.event.id);
      if (previous && !["completed", "cancelled", "failed", "unknown", "skipped", "expired"].includes(previous)
        && ["failed", "unknown"].includes(entry.status) && entry.error !== status?.last_error) {
        feedback.error("Agent 事件处理失败", entry.error || "事件处理结果未知，请检查执行端状态。");
      }
    }
    eventStates.current = new Map(status?.events.map(entry => [entry.event.id, entry.status]) ?? []);
  }, [status, feedback]);

  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
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
            feedback.clearIssue("agent:status");
          }
        } catch (error) {
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) {
            setConnectionError(errorMessage(error));
            feedback.reportIssue("agent:status", "Agent 状态读取失败", error);
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
      clearTimeout(timer);
      pollController.current?.abort();
      actionController.current?.abort();
      actionController.current = null;
      revision.current += 1;
    };
  }, [client, pollIntervalMs, feedback]);

  async function beginAction<T>(kind: Action, run: (signal: AbortSignal) => Promise<T>): Promise<T | null> {
    if (actionController.current) return null;
    const controller = new AbortController();
    actionController.current = controller;
    revision.current += 1;
    pollController.current?.abort();
    setPendingAction(kind);
    setActionError(null);
    try {
      const result = await run(controller.signal);
      if (controller.signal.aborted) return null;
      setConnectionError(null);
      return result;
    } catch (error) {
      if (!controller.signal.aborted) { setActionError(errorMessage(error)); feedback.error(`${({ settings: "保存 Agent 设置", pause: "暂停 Agent", resume: "恢复 Agent", events: "提交直播事件" })[kind]}失败`, error); }
      return null;
    } finally {
      if (actionController.current === controller) {
        actionController.current = null;
        setPendingAction(null);
      }
    }
  }

  async function updateStatus(kind: Exclude<Action, "events">, run: (signal: AbortSignal) => Promise<AgentSnapshot>): Promise<boolean> {
    const next = await beginAction(kind, run);
    if (!next) return false;
    setStatus(next);
    if (next.last_error) {
      feedback.clearIssue("agent:runtime"); feedback.reportIssue("agent:runtime", "Agent 操作失败", next.last_error); return false;
    }
    if (next.paused !== (kind !== "resume")) {
      feedback.error("Agent 操作未完成", "返回状态未确认本次操作，请检查 Agent 运行状态后重试。"); return false;
    }
    feedback.success(kind === "settings" ? "Agent 设置已保存" : kind === "pause" ? "Agent 已暂停" : "Agent 已恢复", kind === "settings" ? "新设置已保存，Agent 已暂停；确认配置后可恢复互动。" : "已更新 Agent 运行状态。");
    return true;
  }

  return {
    status,
    connectionError,
    actionError,
    pendingAction,
    saveSettings: (settings: AgentSettings) => updateStatus("settings", (signal) => client.saveSettings(settings, signal)),
    pause: () => updateStatus("pause", (signal) => client.pause(signal)),
    resume: () => updateStatus("resume", (signal) => client.resume(signal)),
    submitEvents: async (batch: EventBatchRequest): Promise<EventBatchResult | null> => {
      const result = await beginAction("events", (signal) => client.submitEvents(batch, signal));
      if (result) {
        const message = result.persisted === undefined
          ? `已接收 ${result.accepted} 条事件，忽略 ${result.duplicates} 条重复事件。后续处理结果请查看事件记录。`
          : `已持久保存 ${result.persisted} 条事件${result.unscheduled ? `，${result.unscheduled} 条未安排回应` : ""}。`;
        feedback.notify({ kind: "info", title: "直播事件已提交", message });
      }
      return result;
    },
  };
}
