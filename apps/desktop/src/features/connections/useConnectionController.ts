import { useEffect, useRef, useState } from "react";
import type { LiveConnectionSnapshot } from "@meowlive/contracts";
import type { LiveClient } from "../../services/server/live";

type Action = "connect" | "disconnect";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "请求失败，请稍后重试。";
}

export function useConnectionController(client: LiveClient, pollIntervalMs: number) {
  const [status, setStatus] = useState<LiveConnectionSnapshot | null>(null);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<Action | null>(null);
  const pollController = useRef<AbortController | null>(null);
  const actionController = useRef<AbortController | null>(null);
  const revision = useRef(0);
  const mounted = useRef(false);

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
      mounted.current = false;
      clearTimeout(timer);
      pollController.current?.abort();
      actionController.current?.abort();
      actionController.current = null;
      revision.current += 1;
    };
  }, [client, pollIntervalMs]);

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
      return true;
    } catch (error) {
      if (!controller.signal.aborted && mounted.current) setActionError(errorMessage(error));
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
