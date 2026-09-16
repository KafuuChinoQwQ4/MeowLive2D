import { useEffect, useRef, useState } from "react";
import type { ServerStatus, SpeechRequest, SpeechStatus } from "@meowlive/contracts";
import type { ServerClient } from "../../services/server";

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
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<"speech" | "stop" | null>(null);
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
            setStatus(boundHistory(next));
            setConnectionError(null);
          }
        } catch (error) {
          if (!disposed && !controller.signal.aborted && startedRevision === revision.current) setConnectionError(errorMessage(error));
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
        setStatus((current) => current ? boundHistory({
          ...current,
          speeches: [...current.speeches.filter((entry) => entry.id !== snapshot.id), snapshot],
        }) : current);
      } else {
        const next = await client.stop(controller.signal);
        if (controller.signal.aborted) return false;
        setStatus(boundHistory(next));
      }
      setConnectionError(null);
      return true;
    } catch (error) {
      if (!controller.signal.aborted) setActionError(errorMessage(error));
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
