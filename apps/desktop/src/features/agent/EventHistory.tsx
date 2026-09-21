import type { AgentEventSnapshot, AgentEventStatus } from "@meowlive/contracts";

const statusLabels: Record<AgentEventStatus, string> = {
  pending: "等待处理", deciding: "正在决策", skipped: "已忽略", expired: "已过期",
  queued: "排队中", synthesizing: "合成中", ready: "等待播放", playing: "播放中",
  completed: "已完成", cancelled: "已取消", failed: "失败", unknown: "结果未知",
};

function eventLabel(kind: AgentEventSnapshot["event"]["kind"]): string {
  switch (kind.type) {
    case "chat": return kind.text;
    case "gift": return `${kind.name} × ${kind.count}`;
    case "super_chat": return `SC · ${kind.amount_cny} 元 · ${kind.text}`;
    case "room_enter": return "进入直播间";
  }
}

export function EventHistory({ events }: { events: AgentEventSnapshot[] }) {
  const items = events.slice().reverse();
  return (
    <section className="panel history-panel" aria-labelledby="agent-history-heading">
      <div className="section-title"><h2 id="agent-history-heading">事件记录</h2><span className="field-hint">{events.length} 条</span></div>
      {items.length === 0 ? <div className="empty-state"><p>还没有直播事件</p><span>发送模拟事件开始体验。</span></div> :
        <ol className="speech-list" aria-label="Agent 事件">
          {items.map((item) => <li key={`${item.event.source}:${item.event.id}`}>
            <div className="task-heading">
              <span className={`task-status task-${item.status}`}>{statusLabels[item.status]}</span>
              <span className="field-hint">{item.event.viewer} · {item.event.source}</span>
            </div>
            <p className="speech-text">{eventLabel(item.event.kind)}</p>
            {item.speech_id && <p className="field-hint">关联播报 {item.speech_id}</p>}
            {item.error && <p className="field-error">{item.error}</p>}
          </li>)}
        </ol>}
    </section>
  );
}
