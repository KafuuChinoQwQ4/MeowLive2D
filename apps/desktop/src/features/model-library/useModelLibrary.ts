import { useEffect, useRef, useState } from "react";
import type { ModelLibrarySnapshot } from "@meowlive/contracts";
import type { ModelLibraryClient } from "../../services/model-library";

export function useModelLibrary(client: ModelLibraryClient, token: string | null, onSelected: () => void) {
  const [snapshot, setSnapshot] = useState<ModelLibrarySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stale, setStale] = useState(true);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const action = useRef<AbortController | null>(null);
  const polling = useRef<AbortController | null>(null);
  const sequence = useRef(0);
  const mounted = useRef(false);
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
          if (!cancelled && !controller.signal.aborted && generation === sequence.current) { setSnapshot(next); setError(null); setStale(false); }
        } catch (failure) {
          if (!cancelled && !controller.signal.aborted && generation === sequence.current) { setError(failure instanceof Error ? failure.message : "模型状态读取失败"); setStale(true); }
        } finally { if (polling.current === controller) polling.current = null; }
      }
      if (!cancelled) timer = setTimeout(() => void poll(), 2500);
    }
    void poll();
    return () => { cancelled = true; mounted.current = false; clearTimeout(timer); polling.current?.abort(); action.current?.abort(); action.current = null; };
  }, [client, attempt]);
  async function run(kind: "scan" | "select" | "download" | "cancel", id?: string) {
    if (!token || action.current || stale || !snapshot?.environment.ready) return;
    const controller = new AbortController();
    action.current = controller; sequence.current += 1; polling.current?.abort(); setBusy(true); setError(null);
    try {
      const next = kind === "scan" ? await client.scan(token, controller.signal) : await client[kind](id!, token, controller.signal);
      if (mounted.current && !controller.signal.aborted) { setSnapshot(next); setStale(false); if (kind === "select") onSelected(); }
    } catch (failure) {
      if (mounted.current && !controller.signal.aborted) { setError(`${failure instanceof Error ? failure.message : "模型操作失败"} 请刷新状态确认结果。`); setStale(true); }
    } finally { if (action.current === controller) action.current = null; if (mounted.current && !controller.signal.aborted) setBusy(false); }
  }
  return { snapshot, error, stale, busy, run, refresh: () => setAttempt(value => value + 1) };
}
