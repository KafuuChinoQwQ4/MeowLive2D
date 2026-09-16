import { useCallback, useEffect, useRef, useState } from "react";
import type { ModelVersion, ResourceSnapshot, RuntimePresetSnapshot, TrainingSnapshot } from "@meowlive/contracts";
import type { TrainingClient } from "../../services/server/training";
import type { ResourcesClient } from "../../services/server/resources";

type Query = "training" | "resources" | "preset";
const queries: Query[] = ["training", "resources", "preset"];
const emptyErrors = { training: "", resources: "", preset: "" };

export function useTraining(client: TrainingClient, resources: ResourcesClient) {
  const [snapshot, setSnapshot] = useState<TrainingSnapshot | null>(null);
  const [assets, setAssets] = useState<ResourceSnapshot | null>(null);
  const [preset, setPreset] = useState<RuntimePresetSnapshot | null>(null);
  const [operationError, setError] = useState("");
  const [queryErrors, setQueryErrors] = useState(emptyErrors);
  const [pending, setPending] = useState(false);
  const [audio, setAudio] = useState<{ version: string; url: string } | null>(null);
  const context = useRef<{ ctrl: AbortController; revisions: Record<Query, number>; pending: boolean } | null>(null);
  const blob = useRef<string | null>(null);
  const refreshQuery = useCallback(async (query: Query) => {
    const current = context.current;
    if (!current || current.ctrl.signal.aborted) return;
    const revision = ++current.revisions[query];
    const isCurrent = () => context.current === current && !current.ctrl.signal.aborted && revision === current.revisions[query];
    try {
      if (query === "training") {
        const value = await client.snapshot(current.ctrl.signal);
        if (!isCurrent()) return;
        setSnapshot(value);
      } else if (query === "resources") {
        const value = await resources.getSnapshot(current.ctrl.signal);
        if (!isCurrent()) return;
        setAssets(value);
      } else {
        const value = await client.preset(current.ctrl.signal);
        if (!isCurrent()) return;
        setPreset(value);
      }
      setQueryErrors(errors => ({ ...errors, [query]: "" }));
    } catch (e) {
      if (isCurrent()) setQueryErrors(errors => ({ ...errors, [query]: e instanceof Error ? e.message : "读取训练状态失败" }));
    }
  }, [client, resources]);
  const refresh = useCallback(async () => {
    // Resource and preset latency must not hold training controls pending.
    void refreshQuery("resources");
    void refreshQuery("preset");
    await refreshQuery("training");
  }, [refreshQuery]);
  useEffect(() => {
    const current = { ctrl: new AbortController(), revisions: { training: 0, resources: 0, preset: 0 }, pending: false };
    context.current = current;
    setSnapshot(null); setAssets(null); setPreset(null); setPending(false); setAudio(null);
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
      for (const query of queries) clearTimeout(timers[query]);
      if (blob.current) URL.revokeObjectURL(blob.current);
      blob.current = null;
    };
  }, [refreshQuery]);
  async function action(work: (signal: AbortSignal) => Promise<void>) {
    const current = context.current;
    if (!current || current.pending || current.ctrl.signal.aborted) return;
    current.pending = true;
    for (const query of queries) current.revisions[query]++;
    setPending(true); setError("");
    try { await work(current.ctrl.signal); if (!current.ctrl.signal.aborted) await refresh(); }
    catch (e) { if (!current.ctrl.signal.aborted) setError(e instanceof Error ? e.message : "训练操作失败"); }
    finally { current.pending = false; if (!current.ctrl.signal.aborted) setPending(false); }
  }
  async function audition(version: string, text: string) {
    await action(async signal => {
      const value = await client.audition(version, text, signal);
      if (signal.aborted) return;
      if (blob.current) URL.revokeObjectURL(blob.current);
      blob.current = URL.createObjectURL(value); setAudio({ version, url: blob.current });
    });
  }
  async function saveVersion(id: string) {
    await action(async signal => {
      const value = await client.save(id, signal);
      if (signal.aborted) return;
      context.current!.revisions.training++;
      setSnapshot(value);
    });
  }
  async function activateVersion(version: ModelVersion) {
    await action(async signal => {
      const value = await client.activate(version.id, signal);
      if (signal.aborted) return;
      context.current!.revisions.training++;
      setSnapshot(value);
      const selected = await resources.selectVoice({ id: version.voice_id }, signal);
      if (signal.aborted) return;
      context.current!.revisions.resources++;
      setAssets(selected);
    });
  }
  const error = [operationError, ...queries.map(query => queryErrors[query])].filter(Boolean).join("；");
  return { snapshot, voices: assets?.voices ?? [], activeVoiceId: assets?.active_voice_id, defaultVoiceAvailable: assets?.default_voice_available ?? false, preset, error, setError, pending, audio, action, audition, saveVersion, activateVersion, refresh };
}
