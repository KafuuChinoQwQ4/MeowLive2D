import { useEffect, useRef, useState } from "react";
import type { LauncherServiceId, LauncherSnapshot } from "@meowlive/contracts";
import type { LauncherClient } from "../../services/launcher";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export function useLauncher(client: LauncherClient) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<LauncherSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stale, setStale] = useState(true);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const sequence = useRef(0);
  const poll = useRef<AbortController | null>(null);
  const action = useRef<AbortController | null>(null);
  const mounted = useRef(false);
  const refreshRequested = useRef(false);
  const serviceNames = { server: "主服务", tts: "TTS 语音引擎", windows: "Windows 执行端" };

  useEffect(() => {
    for (const service of snapshot?.services ?? []) {
      const key = `launcher:service:${service.id}`;
      if (service.state === "failed") feedback.reportIssue(key, `${serviceNames[service.id]}运行失败`, service.message);
      else feedback.clearIssue(key);
    }
  }, [snapshot, feedback]);

  useEffect(() => {
    mounted.current = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let cancelled = false;
    async function update() {
      if (cancelled) return;
      if (!action.current) {
        const generation = sequence.current;
        const controller = new AbortController();
        poll.current = controller;
        try {
          const value = await client.getStatus(controller.signal);
          if (!cancelled && !controller.signal.aborted && generation === sequence.current) {
            setSnapshot(value); setStale(false); setError(null);
            feedback.clearIssue("launcher:status");
            if (refreshRequested.current) { refreshRequested.current = false; }
          }
        } catch (failure) {
          if (!cancelled && !controller.signal.aborted && generation === sequence.current) {
            setError(failure instanceof Error ? failure.message : "启动管理状态读取失败"); setStale(true);
            if (refreshRequested.current) { refreshRequested.current = false; feedback.error("刷新启动管理状态失败", failure); }
            else feedback.reportIssue("launcher:status", "启动管理状态读取失败", failure);
          }
        } finally { if (poll.current === controller) poll.current = null; }
      }
      if (!cancelled) timer = setTimeout(() => void update(), 1500);
    }
    void update();
    return () => { cancelled = true; mounted.current = false; clearTimeout(timer); poll.current?.abort(); action.current?.abort(); action.current = null; };
  }, [client, attempt, feedback]);

  async function setEnabled(id: LauncherServiceId, enabled: boolean) {
    if (!snapshot || stale || action.current) return;
    const controller = new AbortController();
    action.current = controller;
    sequence.current += 1;
    poll.current?.abort();
    setBusy(true); setError(null);
    try {
      const value = await client.setEnabled(id, enabled, snapshot.session_token, controller.signal);
      if (mounted.current && !controller.signal.aborted) {
        setSnapshot(value); setStale(false);
        const service = value.services.find(item => item.id === id);
        if (service?.state === "failed") {
          feedback.clearIssue(`launcher:service:${id}`);
          feedback.reportIssue(`launcher:service:${id}`, `${serviceNames[id]}操作失败`, service.message);
        }
      }
    } catch (failure) {
      if (mounted.current && !controller.signal.aborted) {
        setError(`${failure instanceof Error ? failure.message : "启停失败"} 请刷新状态确认结果。`); setStale(true);
        feedback.error(`${serviceNames[id]}${enabled ? "启动" : "停止"}失败`, failure);
      }
    } finally {
      if (action.current === controller) action.current = null;
      if (mounted.current && !controller.signal.aborted) setBusy(false);
    }
  }
  return { snapshot, error, stale, busy, setEnabled, refresh: () => { refreshRequested.current = true; setAttempt(value => value + 1); }, refreshQuietly: () => setAttempt(value => value + 1) };
}
