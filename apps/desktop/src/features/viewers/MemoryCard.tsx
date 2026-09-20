import { useState } from "react";
import type { ViewerMemory } from "@meowlive/contracts";
import type { MemoryClient } from "../../services/server/memory";
import type { WriteAction } from "./ViewerDetail";
const states: Record<string, string> = {
  candidate: "候选",
  short_term: "短期",
  long_term: "长期",
  expired: "过期",
  Candidate: "候选",
  ShortTerm: "短期",
  LongTerm: "长期",
  Expired: "过期",
};
export function MemoryCard({
  memory: m,
  client,
  write,
  busy,
  onDelete,
}: {
  memory: ViewerMemory;
  client: MemoryClient;
  write: WriteAction;
  busy: boolean;
  onDelete: () => void;
}) {
  const [value, setValue] = useState(m.value);
  const [reason, setReason] = useState("");
  const mutate = (operation: string) =>
    void write(
      JSON.stringify([
        "memory",
        m.id,
        m.version,
        operation,
        value,
        reason,
        m.locked,
      ]),
      (key, signal) =>
        client.mutate(
          m.viewer_id,
          m.id,
          {
            request_key: key,
            reason,
            expected_version: m.version,
            operation,
            value: operation === "correct" ? value : null,
            frozen: operation === "freeze" ? !m.locked : null,
          },
          signal,
        ),
      operation === "delete" ? onDelete : undefined,
    );
  return (
    <article className="history-row" aria-label={`记忆 ${m.value}`}>
      <strong>
        {states[m.status] ?? m.status} · 版本 {m.version} ·{" "}
        {m.locked ? "已冻结" : "自动更新"}
      </strong>
      <p>{m.value}</p>
      <p>
        到期：
        {m.expires_at_ms === null
          ? "无自动期限"
          : new Date(m.expires_at_ms).toLocaleString()}
      </p>
      {m.evidence.map((e, i) => (
        <blockquote key={`${e.source}:${e.event_id}:${i}`}>
          {e.quote}
          <small>
            {" "}
            · {e.source} / {e.event_id} ·{" "}
            {new Date(e.occurred_at_ms).toLocaleString()}
          </small>
        </blockquote>
      ))}
      <label>
        纠正记忆内容
        <textarea
          value={value}
          maxLength={512}
          onChange={(e) => setValue(e.target.value)}
        />
      </label>
      <label>
        记忆操作原因
        <input
          value={reason}
          maxLength={1000}
          onChange={(e) => setReason(e.target.value)}
        />
      </label>
      <div className="form-actions">
        <button
          type="button"
          disabled={busy || !reason.trim() || !value.trim()}
          onClick={() => mutate("correct")}
        >
          确认纠正
        </button>
        <button
          type="button"
          disabled={busy || !reason.trim()}
          onClick={() => mutate("freeze")}
        >
          {m.locked ? "解除冻结" : "冻结记忆"}
        </button>
        <button
          type="button"
          disabled={busy || !reason.trim()}
          onClick={() => mutate("delete")}
        >
          确认删除记忆
        </button>
      </div>
    </article>
  );
}
