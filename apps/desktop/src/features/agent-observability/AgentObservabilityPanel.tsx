import { useEffect, useRef, useState } from "react";
import type { AgentSchedulerSnapshot, AgentTrace, AgentTraceList, AgentTraceStatus } from "@meowlive/contracts";
import type { AgentObservabilityClient } from "../../services/server/agent-observability";
import { useRuntimeVisibility } from "../llm-runtime/useRuntimeVisibility";
import "./agent-observability.css";

const statusLabels: Record<AgentTraceStatus, string> = {
  running: "进行中", completed: "已完成", failed: "失败", cancelled: "已取消", interrupted: "进程中断",
};
const stepLabels: Record<string, string> = {
  scheduled: "已调度", context_ready: "上下文", turn_started: "模型调用", first_token: "首段输出",
  turn_finished: "模型返回", tool_started: "工具开始", tool_finished: "工具结束", decision_received: "最终决策",
  validation_finished: "业务校验", speech_queued: "语音排队", speech_synthesizing: "语音合成",
  speech_ready: "等待播放", speech_playing: "正在播放", trace_finished: "任务结束",
};

export function AgentObservabilityPanel({ client, pollIntervalMs = 1_000 }: { client: AgentObservabilityClient; pollIntervalMs?: number }) {
  const { ref, visible } = useRuntimeVisibility();
  const [scheduler, setScheduler] = useState<AgentSchedulerSnapshot | null>(null);
  const [list, setList] = useState<AgentTraceList | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [trace, setTrace] = useState<AgentTrace | null>(null);
  const [error, setError] = useState(false);
  const selectedRef = useRef<string | null>(null);
  const traceRef = useRef<AgentTrace | null>(null);
  const selectionRequest = useRef<AbortController | null>(null);
  const detailVersion = useRef(0);
  const explicitSelection = useRef(false);
  useEffect(() => { selectedRef.current = selected; }, [selected]);
  useEffect(() => { traceRef.current = trace; }, [trace]);
  useEffect(() => {
    selectedRef.current = null; traceRef.current = null; explicitSelection.current = false;
    detailVersion.current += 1;
    setScheduler(null); setList(null); setSelected(null); setTrace(null); setError(false);
  }, [client]);
  useEffect(() => {
    if (!visible) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function refresh() {
      try {
        const [nextScheduler, nextList] = await Promise.all([
          client.getScheduler(controller.signal), client.listTraces({ limit: 50 }, controller.signal),
        ]);
        if (controller.signal.aborted) return;
        setScheduler(nextScheduler); setList(nextList); setError(false);
        const id = selectedRef.current && explicitSelection.current
          ? selectedRef.current : nextList.traces[0]?.id ?? null;
        if (id && id !== selectedRef.current) { selectedRef.current = id; setSelected(id); }
        const listedTrace = nextList.traces.find(item => item.id === id);
        if (id && (!traceRef.current || traceRef.current.summary.id !== id
          || (listedTrace ? traceRef.current.summary.updated_at_ms !== listedTrace.updated_at_ms : traceRef.current.summary.status === "running"))) {
          const version = ++detailVersion.current;
          const nextTrace = await client.getTrace(id, controller.signal);
          if (!controller.signal.aborted && selectedRef.current === id && version === detailVersion.current) {
            traceRef.current = nextTrace; setTrace(nextTrace);
          }
        } else if (!id) setTrace(null);
      } catch { if (!controller.signal.aborted) setError(true); }
      if (!controller.signal.aborted) timer = setTimeout(() => { void refresh(); }, pollIntervalMs);
    }
    void refresh();
    return () => { controller.abort(); selectionRequest.current?.abort(); clearTimeout(timer); };
  }, [client, pollIntervalMs, visible]);

  async function selectTrace(id: string) {
    const version = ++detailVersion.current;
    explicitSelection.current = true;
    selectedRef.current = id; traceRef.current = null; setSelected(id); setTrace(null);
    selectionRequest.current?.abort();
    const controller = new AbortController();
    selectionRequest.current = controller;
    try {
      const nextTrace = await client.getTrace(id, controller.signal);
      if (!controller.signal.aborted && selectedRef.current === id && version === detailVersion.current) {
        traceRef.current = nextTrace; setTrace(nextTrace); setError(false);
      }
    } catch { if (!controller.signal.aborted && version === detailVersion.current) setError(true); }
  }

  return <div ref={ref} className="agent-observability">
    <section className="panel agent-observability-scheduler" aria-labelledby="agent-scheduler-heading">
      <div className="agent-observability-heading"><div><p className="eyebrow">实时准入判断</p><h2 id="agent-scheduler-heading">调度状态</h2></div>
        <span className={`agent-trace-badge ${scheduler?.ready ? "completed" : "running"}`} role="status">{scheduler?.ready ? "可以调度" : "正在等待"}</span></div>
      {scheduler ? <><p className="agent-observability-lead">{scheduler.message}</p>
        <dl className="agent-observability-metrics"><div><dt>待处理事件</dt><dd>{scheduler.pending_events}</dd></div>
          <div><dt>决策中事件</dt><dd>{scheduler.deciding_events}</dd></div><div><dt>活动语音</dt><dd>{scheduler.active_speeches}</dd></div>
          <div><dt>Agent 阶段</dt><dd>{scheduler.phase}</dd></div></dl>
        {scheduler.remaining_ms !== null && <p className="field-hint">预计还需 {(scheduler.remaining_ms / 1000).toFixed(1)} 秒结束冷却。</p>}</>
        : <p className="muted">正在读取调度状态…</p>}
      {error && <p className="field-error" role="alert">观察数据暂时无法更新，连接恢复后会自动重试。</p>}
    </section>

    <div className="agent-observability-layout">
      <section className="panel agent-trace-browser" aria-labelledby="agent-traces-heading">
        <div className="agent-observability-heading"><h2 id="agent-traces-heading">Trace 历史</h2><span>{list?.traces.length ?? 0} 条</span></div>
        {list && !list.storage_available && <p className="availability-note">历史持久化暂不可用，当前列表可能不完整。</p>}
        {list?.truncated && <p className="field-hint">列表只显示最近一部分记录。</p>}
        {list && list.traces.length === 0 && <p className="muted">还没有 Agent 调用记录。</p>}
        <ol className="agent-trace-list">{list?.traces.map(item => <li key={item.id}>
          <button type="button" aria-current={selected === item.id ? "true" : undefined} onClick={() => void selectTrace(item.id)}
            aria-label={`${item.id} ${statusLabels[item.status]}`}>
            <span className="agent-trace-row"><strong>{item.trigger === "proactive" ? "主动发言" : `${item.event_count} 个直播事件`}</strong>
              <span className={`agent-trace-badge ${item.status}`}>{statusLabels[item.status]}</span></span>
            <span className="agent-trace-id">{item.id}</span><span><time dateTime={new Date(item.started_at_ms).toISOString()}>{new Date(item.started_at_ms).toLocaleString("zh-CN")}</time> · {item.turn_count} turn · {item.tool_count} 工具 · {duration(item)}</span>
          </button></li>)}</ol>
      </section>

      <section className="panel agent-trace-detail" aria-live="polite">
        {!trace && <p className="muted">选择一条 Trace 查看完整调用过程。</p>}
        {trace && <><div className="agent-observability-heading"><div><p className="eyebrow">{statusLabels[trace.summary.status]}</p>
          <h2>Trace {trace.summary.id}</h2></div><span>{duration(trace.summary)}</span></div>
          <p className="agent-observability-lead">{trace.summary.result}</p>
          {trace.summary.truncated && <p className="availability-note">这条 Trace 达到记录上限，时间线已截断。</p>}
          {trace.events.length > 0 && <div className="agent-trace-events"><h3>触发事件</h3>{trace.events.map(event => <p key={event.id}><strong>{event.viewer || "观众"}</strong> · {event.summary}</p>)}</div>}
          {trace.turns.length > 0 && <div className="agent-trace-turns"><h3>模型 Turns</h3>{trace.turns.map(turn => <article key={turn.id}>
            <div className="agent-trace-row"><strong>第 {turn.index} 次模型调用</strong><span className={`agent-trace-badge ${turn.status}`}>{statusLabels[turn.status]}</span></div>
            <p>{turn.provider} · {turn.model} · 工具轮次 {turn.tool_round} · 重试 {turn.retry_attempt}</p>
            <p>耗时 {(turn.latency_ms / 1000).toFixed(2)} 秒{turn.first_token_ms !== null ? ` · 首段 ${(turn.first_token_ms / 1000).toFixed(2)} 秒` : ""}
              {` · 输入 ${turn.usage.input_tokens ?? "未报告"} · 输出 ${turn.usage.output_tokens ?? "未报告"}`}</p>
          </article>)}</div>}
          <div className="agent-trace-timeline"><h3>执行时间线</h3><ol>{trace.steps.map(step => <li key={step.sequence} className={step.status}>
            <div className="agent-trace-row"><strong>{stepLabels[step.kind]}</strong><time dateTime={new Date(step.occurred_at_ms).toISOString()}>{new Date(step.occurred_at_ms).toLocaleTimeString("zh-CN")}</time></div>
            <p><span>{step.message}</span>{step.elapsed_ms !== null && ` · ${(step.elapsed_ms / 1000).toFixed(2)} 秒`}</p>
            {step.sources.length > 0 && <div className="agent-trace-sources"><span>来源</span>{step.sources.map(source => <a key={source} href={source} target="_blank" rel="noopener noreferrer" referrerPolicy="no-referrer">{new URL(source).hostname}</a>)}</div>}
          </li>)}</ol></div></>}
      </section>
    </div>
  </div>;
}

function duration(summary: { started_at_ms: number; updated_at_ms: number; finished_at_ms: number | null }) {
  return `${(Math.max(0, (summary.finished_at_ms ?? summary.updated_at_ms) - summary.started_at_ms) / 1000).toFixed(1)} 秒`;
}
