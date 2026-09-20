import { useEffect, useRef, useState } from "react";
import type {
  CompanionshipDetail,
  ViewerMemory,
  CompanionshipHealth,
  MemoryJobsStatus,
} from "@meowlive/contracts";
import {
  createCompanionshipClient,
  type CompanionshipClient,
} from "../../services/server/companionship";
import {
  createMemoryClient,
  type MemoryClient,
} from "../../services/server/memory";
import { ServerRequestError } from "../../services/server/responses";
import { RelationshipPanel } from "./RelationshipPanel";
import { ViewerMergePanel } from "./ViewerMergePanel";
import type { RelationshipClient } from "../../services/server/relationships";
import type { ViewerMergeClient } from "../../services/server/viewerMerge";
import { MemoryCard } from "./MemoryCard";
const defaultCompanionship = createCompanionshipClient();
const defaultMemory = createMemoryClient();
export type WriteAction = (
  fingerprint: string,
  run: (key: string, signal: AbortSignal) => Promise<unknown>,
  done?: () => void,
) => Promise<void>;
export function ViewerDetail({
  viewerId,
  companionship = defaultCompanionship,
  memory = defaultMemory,
  relationships,
  merge,
  onMerged,
}: {
  viewerId: string;
  companionship?: CompanionshipClient;
  memory?: MemoryClient;
  relationships?: RelationshipClient;
  merge?: ViewerMergeClient;
  onMerged?: (id: string) => void;
}) {
  const [detail, setDetail] = useState<CompanionshipDetail | null>(null);
  const [memories, setMemories] = useState<ViewerMemory[]>([]);
  const [health, setHealth] = useState<CompanionshipHealth | null>(null);
  const [jobs, setJobs] = useState<MemoryJobsStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [revision, setRevision] = useState(0);
  const [conflict, setConflict] = useState(false);
  const [maintenanceReason, setMaintenanceReason] = useState("");
  const [delta, setDelta] = useState("");
  const [reason, setReason] = useState("");
  const lifetime = useRef(new AbortController());
  const requestKey = useRef<{ fingerprint: string; key: string } | null>(null);
  useEffect(() => {
    const c = new AbortController();
    lifetime.current = c;
    requestKey.current = null;
    setDelta("");
    setReason("");
    setBusy(false);
    setConflict(false);
    return () => c.abort();
  }, [viewerId]);
  useEffect(() => {
    const c = new AbortController();
    setLoading(true);
    setError(null);
    setDetail(null);
    setMemories([]);
    setHealth(null);
    setJobs(null);
    const report = (e: unknown) => {
      if (!c.signal.aborted)
        setError(e instanceof Error ? e.message : "无法读取管理详情。");
    };
    void Promise.allSettled([
      companionship
        .detail(viewerId, c.signal)
        .then((v) => {
          if (!c.signal.aborted) setDetail(v);
        })
        .catch(report),
      memory
        .list(viewerId, c.signal)
        .then((v) => {
          if (!c.signal.aborted)
            setMemories(v.memories.filter((m) => !m.deleted));
        })
        .catch(report),
      companionship
        .status(c.signal)
        .then((v) => {
          if (!c.signal.aborted) setHealth(v);
        })
        .catch(report),
      memory
        .status(c.signal)
        .then((v) => {
          if (!c.signal.aborted) setJobs(v);
        })
        .catch(report),
    ]).then(() => {
      if (!c.signal.aborted) setLoading(false);
    });
    return () => c.abort();
  }, [viewerId, companionship, memory, revision]);
  const write: WriteAction = async (fingerprint, run, done) => {
    if (busy || conflict) return;
    const c = lifetime.current;
    if (requestKey.current?.fingerprint !== fingerprint)
      requestKey.current = { fingerprint, key: crypto.randomUUID() };
    setBusy(true);
    setError(null);
    try {
      await run(requestKey.current.key, c.signal);
      if (!c.signal.aborted) {
        done?.();
        requestKey.current = null;
        setRevision((v) => v + 1);
      }
    } catch (e) {
      if (!c.signal.aborted) {
        if (e instanceof ServerRequestError && e.status === 409)
          setConflict(true);
        setError(
          e instanceof ServerRequestError && e.status === 409
            ? "资料版本冲突，请刷新后重新操作。"
            : e instanceof Error
              ? e.message
              : "操作失败，可按原内容重试。",
        );
      }
    } finally {
      if (!c.signal.aborted) setBusy(false);
    }
  };
  return (
    <section
      className="panel viewer-detail"
      aria-label="观众管理详情"
    >
      <h3>观众管理详情</h3>
      <p className="muted">观众档案 · {viewerId}</p>
      <button
        type="button"
        disabled={loading || busy}
        onClick={() => {
          setConflict(false);
          if (conflict) requestKey.current = null;
          setRevision((v) => v + 1);
        }}
      >
        刷新详情与任务
      </button>
      {loading && <p role="status">正在读取管理详情…</p>}
      {error && (
        <p role="alert" className="error-banner">
          {error}
        </p>
      )}
      {health && (
        <>
          <p>
            {health.durable_receipts
              ? "回执恢复：本地持久日志已启用"
              : "回执恢复：未启用持久日志"}
          </p>
          {health.failed_receipts.map((r) => (
            <p key={r.speech_id}>
              待恢复播报 {r.speech_id} · 失败尝试 {r.attempts}
            </p>
          ))}
        </>
      )}
      {health && (
        <p>
          待持久回执 {health.pending_receipts} · 失败尝试{" "}
          {health.failed_receipt_attempts}
        </p>
      )}
      {jobs && (
        <p>
          提取任务：待处理 {jobs.pending} · 运行 {jobs.running} · 失败{" "}
          {jobs.failed} · 待嵌入 {jobs.embedding_pending} · 嵌入失败{" "}
          {jobs.embedding_failed}
        </p>
      )}
      <label>
        记忆恢复操作原因
        <input
          value={maintenanceReason}
          maxLength={1000}
          onChange={(e) => setMaintenanceReason(e.target.value)}
        />
      </label>
      <button
        type="button"
        disabled={busy || conflict || !maintenanceReason.trim()}
        onClick={() =>
          void write(
            JSON.stringify(["memory-retry", maintenanceReason]),
            (key, signal) =>
              memory.retry(
                { request_key: key, reason: maintenanceReason },
                signal,
              ),
          )
        }
      >
        重试失败记忆任务
      </button>
      <button
        type="button"
        disabled={busy || conflict || !maintenanceReason.trim()}
        onClick={() =>
          void write(
            JSON.stringify(["vector-rebuild", maintenanceReason]),
            (key, signal) =>
              memory.rebuildVectors(
                { request_key: key, reason: maintenanceReason },
                signal,
              ),
          )
        }
      >
        重建记忆向量
      </button>
      {relationships && (
        <RelationshipPanel
          key={viewerId}
          viewerId={viewerId}
          client={relationships}
        />
      )}
      {merge && onMerged && (
        <ViewerMergePanel
          key={viewerId}
          viewerId={viewerId}
          client={merge}
          onMerged={onMerged}
        />
      )}
      {detail && (
        <>
          <p>好感度 {detail.affinity_milli / 1000} / 100</p>
          <p>熟悉度 {detail.familiarity_milli / 1000} / 100</p>
          <p>
            互动日 {detail.observed_days} · 观察场次 {detail.observed_sessions}
          </p>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void write(
                JSON.stringify(["adjust", viewerId, delta, reason]),
                (key, signal) =>
                  companionship.adjust(
                    viewerId,
                    { request_key: key, reason, delta_milli: Number(delta) },
                    signal,
                  ),
              );
            }}
          >
            <label>
              调整毫分
              <input
                aria-label="调整毫分"
                type="number"
                min={-100000}
                max={100000}
                step={1}
                required
                value={delta}
                onChange={(e) => setDelta(e.target.value)}
              />
            </label>
            <label>
              调整原因
              <input
                aria-label="调整原因"
                required
                maxLength={1000}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
            </label>
            <button
              disabled={
                busy ||
                conflict ||
                !reason.trim() ||
                !delta ||
                !Number.isSafeInteger(Number(delta))
              }
            >
              提交调整
            </button>
          </form>
          <h4>礼物原始记录</h4>
          {detail.gifts.length === 0 && <p>暂无礼物记录。</p>}
          {detail.gifts.map((g) => (
            <GiftCard
              key={`${g.source}:${g.event_id}`}
              gift={g}
              viewerId={viewerId}
              client={companionship}
              write={write}
              busy={busy || conflict}
            />
          ))}
          <h4>积分依据</h4>
          {detail.ledger.map((l) => (
            <LedgerCard
              key={l.ledger_id}
              entry={l}
              viewerId={viewerId}
              client={companionship}
              write={write}
              busy={busy || conflict}
              reversed={detail.ledger.some(
                (r) => r.reversed_ledger_id === l.ledger_id,
              )}
            />
          ))}
        </>
      )}
      <h4>记忆与证据</h4>
      {!loading && memories.length === 0 && <p>暂无有效管理记录。</p>}
      {memories.map((m) => (
        <MemoryCard
          key={`${m.id}:${m.version}`}
          memory={m}
          client={memory}
          write={write}
          busy={busy || conflict}
          onDelete={() =>
            setMemories((rows) => rows.filter((r) => r.id !== m.id))
          }
        />
      ))}
    </section>
  );
}
function GiftCard({
  gift: g,
  viewerId,
  client,
  write,
  busy,
}: {
  gift: CompanionshipDetail["gifts"][number];
  viewerId: string;
  client: CompanionshipClient;
  write: WriteAction;
  busy: boolean;
}) {
  const [value, setValue] = useState("");
  const [reason, setReason] = useState("");
  return (
    <article className="history-row">
      <strong>
        {g.name} × {g.count}
      </strong>
      <p>
        原始价格 {g.metadata?.price ?? "未知"} · 付费{" "}
        {g.metadata?.paid === true
          ? "是"
          : g.metadata?.paid === false
            ? "否"
            : "未知"}{" "}
        · 已确认付费价值（分） {g.value_cents ?? "未知"}
      </p>
      <small>
        {g.source} / {g.event_id} ·{" "}
        {new Date(g.occurred_at_ms).toLocaleString()}
      </small>
      {g.value_cents === null && g.metadata?.paid !== false && (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void write(
              JSON.stringify([
                "gift",
                viewerId,
                g.source,
                g.event_id,
                value,
                reason,
              ]),
              (key, signal) =>
                client.confirmGift(
                  viewerId,
                  {
                    request_key: key,
                    reason,
                    source: g.source,
                    event_id: g.event_id,
                    value_cents: Number(value),
                    value_kind: "confirmed_paid_value",
                  },
                  signal,
                ),
            );
          }}
        >
          <label>
            确认付费价值（人民币分）
            <input
              type="number"
              min={0}
              step={1}
              required
              value={value}
              onChange={(e) => setValue(e.target.value)}
            />
          </label>
          <label>
            金额确认依据
            <input
              required
              maxLength={1000}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </label>
          <button
            disabled={
              busy ||
              !reason.trim() ||
              !value ||
              !Number.isSafeInteger(Number(value))
            }
          >
            确认金额
          </button>
        </form>
      )}
    </article>
  );
}
function LedgerCard({
  entry: l,
  viewerId,
  client,
  write,
  busy,
  reversed,
}: {
  entry: CompanionshipDetail["ledger"][number];
  viewerId: string;
  client: CompanionshipClient;
  write: WriteAction;
  busy: boolean;
  reversed: boolean;
}) {
  const [reason, setReason] = useState("");
  return (
    <article className="history-row">
      <strong>
        {l.kind} · 计算 {l.computed_delta_milli} 毫分 · 生效{" "}
        {l.applied_delta_milli} 毫分
      </strong>
      <p>
        {l.reason} · {l.actor} · {new Date(l.created_at_ms).toLocaleString()}
      </p>
      {l.reversed_ledger_id && <small>撤销原记录 {l.reversed_ledger_id}</small>}
      {!l.reversible && (
        <p>此记录不可直接撤销；如需纠正，请填写原因进行人工调整。</p>
      )}
      {l.reversible && !reversed && (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void write(
              JSON.stringify(["reverse", viewerId, l.ledger_id, reason]),
              (key, signal) =>
                client.reverse(
                  viewerId,
                  { request_key: key, reason, ledger_id: l.ledger_id },
                  signal,
                ),
            );
          }}
        >
          <label>
            撤销原因
            <input
              required
              maxLength={1000}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </label>
          <button disabled={busy || !reason.trim()}>撤销此项积分</button>
        </form>
      )}
    </article>
  );
}
