import { useEffect, useMemo, useState } from "react";
import type { ViewerEventPage, ViewerPage } from "@meowlive/contracts";
import { createViewerClient, type ViewerClient } from "../../services/server/viewers";
import { ServerRequestError } from "../../services/server/responses";

import { ViewerDetail } from "./ViewerDetail";
import { createCompanionshipClient } from "../../services/server/companionship";
import { createRelationshipClient } from "../../services/server/relationships";
import { createViewerMergeClient } from "../../services/server/viewerMerge";
import { createMemoryClient } from "../../services/server/memory";

const PAGE_SIZE = 50;
const defaultClient = createViewerClient();

function eventLabel(event: ViewerEventPage["events"][number]): string {
  return event.kind.type === "chat" ? event.kind.text : `${event.kind.name} × ${event.kind.count}`;
}

function identityLabel(identity: ViewerPage["viewers"][number]["identities"][number]): string {
  return `${identity.platform} · ${identity.id_kind} · ${identity.external_id}`;
}

function loadErrorMessage(error: unknown): string {
  if (error instanceof ServerRequestError && error.code === "auth_disabled") {
    return "主服务仍在使用旧版访问规则，请更新并重启主服务后重新读取。";
  }
  if (error instanceof ServerRequestError && error.code === "viewer_store_unavailable") {
    return "观众记录存储当前不可用，请检查持久化存储配置和主服务状态。";
  }
  return "无法读取观众记录，请检查主服务状态后重新读取。";
}

export function ViewerPanel({ client = defaultClient }: { client?: ViewerClient }) {
  const [selected, setSelected] = useState<string | null>(null);
  const management = useMemo(() => ({ companionship: createCompanionshipClient({ baseUrl: client.baseUrl }), memory: createMemoryClient({ baseUrl: client.baseUrl }), relationships: createRelationshipClient({ baseUrl: client.baseUrl }), merge: createViewerMergeClient({ baseUrl: client.baseUrl }) }), [client.baseUrl]);
  const [offset, setOffset] = useState(0);
  const [viewers, setViewers] = useState<ViewerPage | null>(null);
  const [events, setEvents] = useState<ViewerEventPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refresh, setRefresh] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setError(null);
    setLoading(true);
    setViewers(null);
    setEvents(null);
    void Promise.all([client.listViewers(offset, controller.signal), client.listEvents(offset, controller.signal)])
      .then(([nextViewers, nextEvents]) => {
        if (!controller.signal.aborted) {
          setViewers(nextViewers);
          setEvents(nextEvents);
        }
      })
      .catch(error => { if (!controller.signal.aborted) setError(loadErrorMessage(error)); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [client, offset, refresh]);

  const canNext = (viewers?.viewers.length ?? 0) === PAGE_SIZE || (events?.events.length ?? 0) === PAGE_SIZE;
  return <section className="viewer-workspace" aria-labelledby="viewer-records-heading">
    <details className="connection-card viewer-collapsible" open>
      <summary><h2 id="viewer-records-heading">观众与事件</h2></summary>
      <div className="viewer-records-body viewer-records-status">
        <p className="server-address">{client.baseUrl}</p>
        <div className="live-status-actions"><span className="connection-pill disconnected">未确认接收缺口 {events ? events.unconfirmed_events : "未读取"}</span><button type="button" className="secondary-button" onClick={() => setRefresh(value => value + 1)} disabled={loading}>刷新记录</button></div>
      </div>
    </details>
    {error && <p className="error-banner" role="alert">{error}<button type="button" className="secondary-button" onClick={() => setRefresh(value => value + 1)}>重新读取</button></p>}
    <div className="panel-grid">
      <details className="panel viewer-collapsible" open><summary><h3>观众列表</h3></summary>
        <div className="viewer-records-body">
        {!viewers && !error && <p role="status">正在读取观众记录…</p>}
        {viewers?.viewers.length === 0 && <p className="muted">尚无已识别观众。</p>}
        {viewers?.viewers.map(viewer => <article key={viewer.viewer_id} className="history-row"><strong>{viewer.current_alias ?? "未命名观众"}</strong><button type="button" aria-label={`管理 ${viewer.current_alias ?? "未命名观众"}`} onClick={() => setSelected(viewer.viewer_id)}>管理详情</button><p>{viewer.aliases.map(alias => alias.alias).join(" · ") || "尚无昵称记录"}</p>{viewer.identities[0] && <small>{identityLabel(viewer.identities[0])}</small>}</article>)}
        </div>
      </details>
      <details className="panel viewer-collapsible" open><summary><h3>已接收事件</h3></summary>
        <div className="viewer-records-body">
        {!events && !error && <p role="status">正在读取事件记录…</p>}
        {events?.events.length === 0 && <p className="muted">暂无已保存事件。</p>}
        {events?.events.map(event => <article key={`${event.source}:${event.event_id}`} className="history-row"><strong>{event.viewer}</strong><p>{eventLabel(event)}</p>{event.gift_metadata?.price !== undefined && <small>原始价格 {event.gift_metadata.price}</small>}</article>)}
        </div>
      </details>
    </div>
    {selected && <><button type="button" onClick={() => setSelected(null)}>关闭观众详情</button><ViewerDetail key={selected} viewerId={selected} onMerged={id => {setSelected(id);setRefresh(v => v + 1);}} {...management} /></>}
    <div className="form-actions"><button type="button" onClick={() => setOffset(value => Math.max(0, value - PAGE_SIZE))} disabled={loading || offset === 0}>上一页</button><button type="button" onClick={() => setOffset(value => value + PAGE_SIZE)} disabled={loading || !canNext}>下一页</button></div>
  </section>;
}
