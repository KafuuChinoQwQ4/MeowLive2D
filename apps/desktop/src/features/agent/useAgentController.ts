import { useEffect, useRef, useState } from "react";
import type { AgentSettings, AgentSnapshot, EventBatchRequest, EventBatchResult } from "@meowlive/contracts";
import type { AgentClient } from "../../services/server/agent";

type Action = "settings" | "pause" | "resume" | "events";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "请求失败，请稍后重试。";
}

export function useAgentController(client: AgentClient, pollIntervalMs: number) {
  const [status, setStatus] = useState<AgentSnapshot | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<Action | null>(null);
  const pollController = useRef<AbortController | null>(null);
  const actionController = useRef<AbortController | null>(null);
  const revision = useRef(0);

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
          }
        } catch (error) {
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) {
            setConnectionError(errorMessage(error));
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
  }, [client, pollIntervalMs]);

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
      if (!controller.signal.aborted) setActionError(errorMessage(error));
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
    submitEvents: (batch: EventBatchRequest): Promise<EventBatchResult | null> => beginAction("events", (signal) => client.submitEvents(batch, signal)),
  };
}
