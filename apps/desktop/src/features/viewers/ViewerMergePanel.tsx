import { useEffect, useRef, useState } from "react";
import type { ViewerMergePreview } from "@meowlive/contracts";
import type { ViewerMergeClient } from "../../services/server/viewerMerge";
import { ServerRequestError } from "../../services/server/responses";
export function ViewerMergePanel({
  viewerId,
  client,
  onMerged,
}: {
  viewerId: string;
  client: ViewerMergeClient;
  onMerged: (id: string) => void;
}) {
  const [target, setTarget] = useState("");
  const [reason, setReason] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const [preview, setPreview] = useState<ViewerMergePreview | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const live = useRef(new AbortController());
  const pending = useRef<AbortController | null>(null);
  const retry = useRef<{ signature: string; key: string } | null>(null);
  useEffect(() => {
    const c = new AbortController();
    live.current = c;
    return () => {
      c.abort();
      pending.current?.abort();
    };
  }, [viewerId]);
  async function showPreview() {
    pending.current?.abort();
    const c = new AbortController();
    pending.current = c;
    setBusy(true);
    setError("");
    setPreview(null);
    setConfirmed(false);
    try {
      const v = await client.preview(
        { source_viewer_id: viewerId, target_viewer_id: target },
        c.signal,
      );
      if (!c.signal.aborted && !live.current.signal.aborted) setPreview(v);
    } catch (e) {
      if (!c.signal.aborted && !live.current.signal.aborted)
        setError(e instanceof Error ? e.message : "预览失败");
    } finally {
      if (!c.signal.aborted && !live.current.signal.aborted) setBusy(false);
    }
  }
  async function apply() {
    if (!preview || !confirmed || !reason.trim() || busy) return;
    const c = live.current;
    const payload = {
      source_viewer_id: viewerId,
      target_viewer_id: target,
      expected_revision: preview.revision,
      fingerprint: preview.fingerprint,
      reason,
      confirmed: true,
    };
    const signature = JSON.stringify(payload);
    if (retry.current?.signature !== signature)
      retry.current = { signature, key: crypto.randomUUID() };
    setBusy(true);
    setError("");
    try {
      const result = await client.apply(
        { ...payload, request_key: retry.current.key },
        c.signal,
      );
      if (!c.signal.aborted) onMerged(result.canonical_viewer_id);
    } catch (e) {
      if (!c.signal.aborted) {
        setError(e instanceof Error ? e.message : "合并失败，可原样重试");
        if (e instanceof ServerRequestError && e.status === 409) {
          setPreview(null);
          setConfirmed(false);
        }
      }
    } finally {
      if (!c.signal.aborted) setBusy(false);
    }
  }
  return (
    <section aria-label="身份合并">
      <h4>身份合并</h4>
      <p>核对平台身份后再合并，同名不代表同一人。</p>
      <label>
        目标观众 UUID
        <input
          value={target}
          disabled={busy}
          onChange={(e) => {
            pending.current?.abort();
            setTarget(e.target.value);
            setPreview(null);
            setConfirmed(false);
          }}
        />
      </label>
      <button
        type="button"
        disabled={busy || !target.trim() || target === viewerId}
        onClick={() => void showPreview()}
      >
        预览身份合并
      </button>
      {error && <p role="alert">{error}</p>}
      {preview && (
        <>
          <p>
            {preview.source.alias ?? preview.source.viewer_id} →{" "}
            {preview.target.alias ?? preview.target.viewer_id}
          </p>
          {[preview.source, preview.target].map((v, i) => (
            <p key={i}>
              {i === 0 ? "来源" : "目标"} {v.viewer_id} · 身份 {v.identities} ·
              事件 {v.events} · 记忆 {v.memories} · 关系 {v.relationships}
            </p>
          ))}
          <p>
            合并后好感度 {preview.resulting_affinity_milli / 1000} · 熟悉度{" "}
            {preview.resulting_familiarity_milli / 1000}
          </p>
          <ul>
            {preview.risks.map((r, i) => (
              <li key={i}>{r}</li>
            ))}
          </ul>
          <label>
            身份合并原因
            <input
              value={reason}
              maxLength={1000}
              onChange={(e) => setReason(e.target.value)}
            />
          </label>
          <label>
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />
            已核对双方身份与合并风险
          </label>
          <button
            type="button"
            disabled={busy || !confirmed || !reason.trim()}
            onClick={() => void apply()}
          >
            执行身份合并
          </button>
        </>
      )}
    </section>
  );
}
