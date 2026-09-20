import { useEffect, useRef, useState } from "react";
import type {
  RelationshipPage,
  GraphStatus,
  ViewerRelationship,
} from "@meowlive/contracts";
import type { RelationshipClient } from "../../services/server/relationships";
import { ServerRequestError } from "../../services/server/responses";
import type { WriteAction } from "./ViewerDetail";
const kindNames: Record<string, string> = {
  mention: "提及",
  acquaintance: "自述认识",
  participated: "共同参与",
  shared_interest: "共同兴趣",
  friend: "朋友",
};
export function RelationshipPanel({
  viewerId,
  client,
}: {
  viewerId: string;
  client: RelationshipClient;
}) {
  const [page, setPage] = useState<RelationshipPage | null>(null);
  const [status, setStatus] = useState<GraphStatus | null>(null);
  const [depth, setDepth] = useState(1);
  const [revision, setRevision] = useState(0);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [conflict, setConflict] = useState(false);
  const [reason, setReason] = useState("");
  const lifetime = useRef(new AbortController());
  const retry = useRef<{ signature: string; key: string } | null>(null);
  useEffect(() => {
    const c = new AbortController();
    lifetime.current = c;
    return () => c.abort();
  }, [viewerId]);
  useEffect(() => {
    const c = new AbortController();
    setPage(null);
    setError("");
    void Promise.all([
      client.list(viewerId, depth, c.signal),
      client.status(c.signal),
    ])
      .then(([p, s]) => {
        if (!c.signal.aborted) {
          setPage(p);
          setStatus(s);
        }
      })
      .catch((e) => {
        if (!c.signal.aborted)
          setError(e instanceof Error ? e.message : "关系读取失败");
      });
    return () => c.abort();
  }, [viewerId, client, depth, revision]);
  const write: WriteAction = async (signature, run, done) => {
    if (busy || conflict) return;
    const c = lifetime.current;
    if (retry.current?.signature !== signature)
      retry.current = { signature, key: crypto.randomUUID() };
    setBusy(true);
    setError("");
    try {
      await run(retry.current.key, c.signal);
      if (!c.signal.aborted) {
        done?.();
        retry.current = null;
        setRevision((v) => v + 1);
      }
    } catch (e) {
      if (!c.signal.aborted) {
        setError(e instanceof Error ? e.message : "关系操作失败");
        if (e instanceof ServerRequestError && e.status === 409)
          setConflict(true);
      }
    } finally {
      if (!c.signal.aborted) setBusy(false);
    }
  };
  return (
    <section aria-label="观众关系图谱">
      <h4>观众关系图谱</h4>
      <label>
        关系范围
        <select
          value={depth}
          disabled={busy}
          onChange={(e) => setDepth(Number(e.target.value))}
        >
          <option value={1}>一跳</option>
          <option value={2}>两跳</option>
        </select>
      </label>
      <button
        type="button"
        disabled={busy}
        onClick={() => {
          setConflict(false);
          setRevision((v) => v + 1);
        }}
      >
        刷新关系与同步状态
      </button>
      {error && (
        <p role="alert">
          {error}
          {conflict && " 请刷新关系版本。"}
        </p>
      )}
      {page?.degraded && <p>图服务降级：使用 SQL 已核对关系</p>}
      {status && (
        <p>
          图连接 {status.connected ? "已连接" : "不可用"} · 待同步{" "}
          {status.pending} · 租约中 {status.leased} · 失败 {status.failed} ·
          最旧等待{" "}
          {status.oldest_pending_age_ms === null
            ? "无"
            : `${Math.floor(status.oldest_pending_age_ms / 1000)}秒`}
        </p>
      )}
      <div style={{ maxHeight: "35vh", overflowY: "auto" }}>
        {page?.relationships
          .filter((r) => !r.deleted)
          .map((r) => (
            <RelationCard
              key={`${r.id}:${r.version}`}
              relation={r}
              client={client}
              write={write}
              busy={busy || conflict}
            />
          ))}
      </div>
      <RelationshipForm
        viewerId={viewerId}
        client={client}
        write={write}
        busy={busy || conflict}
      />
      <label>
        图重建原因
        <input
          value={reason}
          maxLength={1000}
          onChange={(e) => setReason(e.target.value)}
        />
      </label>
      <button
        type="button"
        disabled={busy || conflict || !reason.trim()}
        onClick={() =>
          void write(JSON.stringify(["graph-rebuild", reason]), (key, signal) =>
            client.rebuild({ request_key: key, reason }, signal),
          )
        }
      >
        从权威事实重建图
      </button>
    </section>
  );
}
function RelationCard({
  relation: r,
  client,
  write,
  busy,
}: {
  relation: ViewerRelationship;
  client: RelationshipClient;
  write: WriteAction;
  busy: boolean;
}) {
  const [reason, setReason] = useState("");
  const action = (action: string) =>
    void write(
      JSON.stringify([r.id, r.version, action, reason]),
      (key, signal) =>
        client.change(
          r.id,
          { expected_version: r.version, action, reason, request_key: key },
          signal,
        ),
    );
  return (
    <article className="history-row">
      <div
        role="img"
        aria-label={`${r.source.id} ${kindNames[r.kind] ?? r.kind} ${r.target.id}`}
        style={{
          display: "flex",
          alignItems: "center",
          gap: "1rem",
          flexWrap: "wrap",
        }}
      >
        <span style={{ border: "1px solid currentColor", padding: ".5rem" }}>
          {r.source.kind}: {r.source.id}
        </span>
        <span>→ {kindNames[r.kind] ?? r.kind} →</span>
        <span style={{ border: "1px solid currentColor", padding: ".5rem" }}>
          {r.target.kind}: {r.target.id}
        </span>
      </div>
      <p>
        {r.confirmation === "confirmed" ? "已确认" : "单方声称"} · 版本{" "}
        {r.version}
      </p>
      {r.evidence.map((e, i) => (
        <blockquote key={i}>
          {e.quote}
          <small>
            {" "}
            {e.source} / {e.event_id}
          </small>
        </blockquote>
      ))}
      <label>
        关系操作原因
        <input
          value={reason}
          maxLength={1000}
          onChange={(e) => setReason(e.target.value)}
        />
      </label>
      <div className="form-actions">
        <button
          type="button"
          disabled={busy || !reason.trim() || r.confirmation === "confirmed"}
          onClick={() => action("confirm")}
        >
          确认关系
        </button>
        <button
          type="button"
          disabled={busy || !reason.trim() || r.confirmation !== "confirmed"}
          onClick={() => action("revoke")}
        >
          撤销关系确认
        </button>
        <button
          type="button"
          disabled={busy || !reason.trim()}
          onClick={() => action("delete")}
        >
          删除关系
        </button>
      </div>
    </article>
  );
}
function RelationshipForm({
  viewerId,
  client,
  write,
  busy,
}: {
  viewerId: string;
  client: RelationshipClient;
  write: WriteAction;
  busy: boolean;
}) {
  const [target, setTarget] = useState("");
  const [entity, setEntity] = useState("viewer");
  const [kind, setKind] = useState("mention");
  const [source, setSource] = useState("");
  const [event, setEvent] = useState("");
  const [quote, setQuote] = useState("");
  const [reason, setReason] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  return (
    <details>
      <summary>登记有证据的关系</summary>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const r = {
            source: { kind: "viewer", id: viewerId },
            target: { kind: entity, id: target },
            kind,
            evidence: [{ source, event_id: event, quote }],
            expires_at_ms: null,
            admin_confirmed: confirmed,
            reason,
          };
          void write(JSON.stringify(r), (key, signal) =>
            client.create({ ...r, request_key: key }, signal),
          );
        }}
      >
        <label>
          目标类型
          <select value={entity} onChange={(e) => setEntity(e.target.value)}>
            {["viewer", "topic", "activity", "unresolved"].map((v) => (
              <option key={v}>{v}</option>
            ))}
          </select>
        </label>
        <label>
          关系目标标识
          <input
            required
            value={target}
            onChange={(e) => setTarget(e.target.value)}
          />
        </label>
        <label>
          关系类型
          <select value={kind} onChange={(e) => setKind(e.target.value)}>
            {Object.entries(kindNames).map(([k, v]) => (
              <option key={k} value={k}>
                {v}
              </option>
            ))}
          </select>
        </label>
        <label>
          证据平台
          <input
            required
            value={source}
            onChange={(e) => setSource(e.target.value)}
          />
        </label>
        <label>
          证据事件 ID
          <input
            required
            value={event}
            onChange={(e) => setEvent(e.target.value)}
          />
        </label>
        <label>
          证据原文片段
          <textarea
            required
            value={quote}
            maxLength={500}
            onChange={(e) => setQuote(e.target.value)}
          />
        </label>
        <label>
          登记关系原因
          <input
            required
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
          我已核对并确认关系
        </label>
        <button
          disabled={
            busy ||
            ![target, source, event, quote, reason].every((v) => v.trim())
          }
        >
          登记关系
        </button>
      </form>
    </details>
  );
}
