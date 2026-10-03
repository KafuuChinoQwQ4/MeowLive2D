import { useEffect, useMemo, useRef, useState } from "react";
import type { RuntimeLogClient, RuntimeLogEntry, RuntimeLogLevel, RuntimeLogListResponse } from "../../services/server/logs";
import { filterRuntimeLogs, localRuntimeLogs } from "../../services/server/logs";
import type { AdminSessionClient } from "../../services/server/auth";
import { ServerRequestError } from "../../services/server/responses";
import { AdminGate } from "../../app/AdminGate";
import { useRuntimeVisibility } from "../llm-runtime/useRuntimeVisibility";
import "./logs.css";

const labels: Record<RuntimeLogLevel, string> = { debug: "调试", info: "信息", warn: "警告", error: "错误" };
const sourceLabels: Record<string, string> = { browser: "浏览器", server: "主服务", launcher: "启动器", desktop: "桌面执行端", bridge: "执行端连接" };
const categoryLabels: Record<string, string> = { execution: "播放执行", companionship: "陪伴积分", graph: "关系图", lifecycle: "启动与退出", runtime: "运行时", http: "请求", auth: "认证", agent: "Agent", llm: "大模型", live: "直播", speech: "播报", training: "训练", resources: "角色与音色", memory: "记忆", viewers: "观众", obs: "OBS", launcher: "启动器", server: "主服务", tts: "语音服务", windows: "Windows 执行端", asr: "语音识别", models: "本地模型", frontend: "前端", startup: "启动", bridge: "执行端连接", storage: "日志存储" };
type LogReader = Pick<RuntimeLogClient, "list">;
interface Props { client: RuntimeLogClient; launcherClient?: LogReader; adminClient?: AdminSessionClient; desktopLogs?: RuntimeLogListResponse; pollIntervalMs?: number }
function LoginComplete({ refresh }: { refresh: () => void }) { useEffect(refresh, [refresh]); return <p>管理员会话已就绪。</p>; }

export function RuntimeLogsPanel({ client, launcherClient, adminClient, desktopLogs, pollIntervalMs = 5000 }: Props) {
  const { ref, visible } = useRuntimeVisibility();
  const [level, setLevel] = useState<RuntimeLogLevel | "">("");
  const [source, setSource] = useState("");
  const [category, setCategory] = useState("");
  const [query, setQuery] = useState("");
  const [paused, setPaused] = useState(false);
  const pausedRef = useRef(paused); pausedRef.current = paused;
  const [revision, setRevision] = useState(0);
  const [entries, setEntries] = useState<RuntimeLogEntry[]>([]);
  const [unavailable, setUnavailable] = useState<string[]>([]);
  const [volatileSources, setVolatileSources] = useState<string[]>([]);
  const [needsLogin, setNeedsLogin] = useState(false);
  const [truncated, setTruncated] = useState(false);
  const [loading, setLoading] = useState(false);
  const [updatedAt, setUpdatedAt] = useState<string | null>(null);
  const cache = useRef(new Map<string, RuntimeLogListResponse>());
  const filter = useMemo(() => ({ level: level || undefined, source: source || undefined, category: category || undefined, query: query.trim() || undefined, limit: 1000 }), [level, source, category, query]);
  const refresh = useMemo(() => () => setRevision(value => value + 1), []);
  const displayedEntries = useMemo(() => filterRuntimeLogs([...new Map([...entries, ...(desktopLogs?.entries ?? [])].map(entry => [`${entry.source}:${entry.id}`, entry])).values()], filter), [entries, desktopLogs, filter]);
  useEffect(() => { cache.current.clear(); setEntries([]); }, [client, launcherClient]);
  useEffect(() => {
    if (!visible) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function load() {
      setLoading(true);
      const readers: [string, LogReader][] = [["主服务", client], ...(launcherClient ? [["启动器", launcherClient] as [string, LogReader]] : [])];
      const results = await Promise.allSettled(readers.map(([, reader]) => reader.list(filter, controller.signal)));
      if (controller.signal.aborted) return;
      const failed: string[] = [];
      let login = false;
      results.forEach((result, index) => {
        const name = readers[index][0];
        if (result.status === "fulfilled") cache.current.set(name, result.value);
        else {
          failed.push(name);
          if (name === "主服务" && result.reason instanceof ServerRequestError && [401, 403].includes(result.reason.status ?? 0)) {
            login = true; cache.current.delete(name);
          }
        }
      });
      const snapshots = [...cache.current.entries()];
      const merged = [...snapshots.flatMap(([, value]) => value.entries), ...localRuntimeLogs()];
      const unique = [...new Map(merged.map(entry => [`${entry.source}:${entry.id}`, entry])).values()];
      setEntries(filterRuntimeLogs(unique, filter));
      setTruncated(unique.length > 1000 || snapshots.some(([, value]) => value.truncated));
      setVolatileSources(snapshots.filter(([, value]) => !value.storage_available).map(([name]) => name));
      setUnavailable(failed); setNeedsLogin(login); setUpdatedAt(new Date().toLocaleTimeString()); setLoading(false);
      timer = setTimeout(() => { if (!pausedRef.current) void load(); }, pollIntervalMs);
    }
    void load();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [client, launcherClient, filter, pollIntervalMs, revision, visible]);
  function togglePaused() { if (paused) refresh(); setPaused(value => !value); }
  return <div ref={ref} className="runtime-logs-workspace">
    <section className="panel runtime-logs-filters" aria-label="日志筛选">
      <div className="section-title"><h2>运行日志</h2><span className="field-hint">最多显示 1000 条，最新优先</span></div>
      <div className="runtime-log-filters-grid">
        <label>级别<select value={level} onChange={e => setLevel(e.target.value as RuntimeLogLevel | "")}><option value="">全部级别</option>{Object.entries(labels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label>来源<select value={source} onChange={e => setSource(e.target.value)}><option value="">全部来源</option>{Object.entries(sourceLabels).map(([value, label]) => <option key={value} value={value}>{label}（{value}）</option>)}</select></label>
        <label>分类<select value={category} onChange={e => setCategory(e.target.value)}><option value="">全部分类</option>{Object.entries(categoryLabels).map(([value, label]) => <option key={value} value={value}>{label}（{value}）</option>)}</select></label>
        <label>关键词<input value={query} maxLength={128} onChange={e => setQuery(e.target.value)} placeholder="搜索安全摘要或事件代码" /></label>
      </div>
      <div className="runtime-logs-actions"><button className="secondary-button" onClick={refresh} disabled={loading}>{loading ? "正在刷新…" : "手动刷新"}</button><button className="secondary-button" onClick={togglePaused}>{paused ? "恢复自动刷新" : "暂停自动刷新"}</button><span className="field-hint" role="status">{paused ? "自动刷新已暂停" : "每 5 秒自动刷新"}{updatedAt && ` · 最近刷新 ${updatedAt}`}</span></div>
    </section>
    {needsLogin && adminClient && <AdminGate client={adminClient}><LoginComplete refresh={refresh} /></AdminGate>}
    {unavailable.length > 0 && <p className="availability-note" role="alert">{unavailable.join("、")}日志暂不可用，当前显示其他来源与本地有限缓存；缓存可能不是最新状态。{needsLogin && "主服务日志需要管理员登录。"}</p>}
    {volatileSources.length > 0 && <p className="availability-note">{volatileSources.join("、")}持久化不可用，日志仅保存在当前进程。</p>}
    {desktopLogs && !desktopLogs.storage_available && <p className="availability-note">桌面日志未持久化，仅保留当前会话内观察到的状态变化。</p>}
    {(truncated || desktopLogs?.truncated || entries.length + (desktopLogs?.entries.length ?? 0) > 1000) && <p className="availability-note">日志过多，列表已截断。可缩小筛选范围。</p>}
    <section className="panel runtime-logs-list" aria-label="日志列表">
      <p className="field-hint">统一展示时间、级别、来源、分类与安全摘要；当前 {displayedEntries.length} 条。</p>
      {displayedEntries.length === 0 ? <p className="muted">{loading ? "正在读取日志…" : "暂无符合条件的日志。"}</p> : <div className="runtime-log-rows">{displayedEntries.map(entry => <article className={`runtime-log-row level-${entry.level}`} key={`${entry.source}:${entry.id}`}>
        <time dateTime={entry.timestamp}>{new Date(entry.timestamp).toLocaleString()}</time><strong>{labels[entry.level]}</strong>
        <span title={entry.source}>{sourceLabels[entry.source] ?? entry.source}<small>{entry.source}</small></span><span title={entry.category}>{categoryLabels[entry.category] ?? entry.category}<small>{entry.category}</small></span>
        <code>{entry.code}</code><p>{entry.summary}</p>
      </article>)}</div>}
    </section>
  </div>;
}
