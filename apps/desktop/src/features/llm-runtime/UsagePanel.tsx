import { useEffect, useState, type FormEvent } from "react";
import type { LlmUsageSnapshot, LlmUsageTotals } from "@meowlive/contracts";
import type { LlmRuntimeClient, LlmUsageQuery } from "../../services/server/llm-runtime";

const tokenFields = [["input_tokens", "输入"], ["output_tokens", "输出"], ["cache_read_tokens", "缓存读取"], ["cache_write_tokens", "缓存写入"], ["reasoning_tokens", "推理"]] as const;
const statusLabels: Record<string, string> = { running: "调用中", completed: "已完成", failed: "失败", cancelled: "已取消", interrupted: "已中断" };
const tokens = (value: number | null) => value === null ? "未报告" : value.toLocaleString("zh-CN");
const dollars = (micro: number) => `$${(micro / 1_000_000).toFixed(6)} USD`;
function estimate(totals: LlmUsageTotals) {
  if (!totals.calls) return "暂无调用";
  if (totals.unpriced_calls >= totals.calls) return "未计价 / 未报告用量";
  return `${totals.unpriced_calls > 0 ? "已计价部分 " : "估算 "}${dollars(totals.estimated_cost_microusd)}`;
}
function TokenTotals({ totals }: { totals: LlmUsageTotals }) {
  return <><p className="field-hint">已报告 tokens 合计（缺失部分不计入）</p><dl className="runtime-token-grid">{tokenFields.map(([field, label]) => <div key={field}><dt>{label}</dt><dd>{tokens(totals[field])}</dd></div>)}</dl></>;
}

export function UsagePanel({ client, active, revision }: { client: LlmRuntimeClient; active: boolean; revision: number }) {
  const [snapshot, setSnapshot] = useState<LlmUsageSnapshot | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [since, setSince] = useState("");
  const [until, setUntil] = useState("");
  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const [query, setQuery] = useState<LlmUsageQuery>({});
  const [attempt, setAttempt] = useState(0);
  useEffect(() => { setSnapshot(null); }, [client]);
  useEffect(() => {
    if (!active) return;
    const controller = new AbortController();
    setLoading(true); setError("");
    void client.getUsage(query, controller.signal).then(value => {
      if (!controller.signal.aborted) setSnapshot(value);
    }).catch(reason => { if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : "用量读取失败。"); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [active, client, query, attempt, revision]);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    const sinceMs = since ? new Date(`${since}T00:00:00`).getTime() : undefined;
    let untilMs: number | undefined;
    if (until) { const date = new Date(`${until}T00:00:00`); date.setDate(date.getDate() + 1); untilMs = date.getTime() - 1; }
    if ((sinceMs !== undefined && !Number.isFinite(sinceMs)) || (untilMs !== undefined && !Number.isFinite(untilMs)) || (sinceMs !== undefined && untilMs !== undefined && sinceMs > untilMs)) {
      setError("请选择有效日期，起始日期不能晚于结束日期。"); return;
    }
    setQuery({ since_ms: sinceMs, until_ms: untilMs, provider: provider.trim() || undefined, model: model.trim() || undefined });
  };
  return <div className="runtime-usage">
    <p className="muted">统计 Agent 回复生成与工具后续轮次的模型调用，包括重试、失败和取消时已报告的用量。金额按当前单价估算，实际账单以供应商为准。</p>
    <p className="field-hint">搜索服务费用、连接测试、独立记忆提取和嵌入调用不计入本页。供应商余额请在供应商平台查看。</p>
    <form onSubmit={submit} className="runtime-usage-filter">
      <div className="runtime-fields"><label>起始日期<input type="date" value={since} onChange={event => setSince(event.target.value)} /></label><label>结束日期<input type="date" value={until} onChange={event => setUntil(event.target.value)} /></label>
        <label>筛选服务商<input value={provider} maxLength={64} placeholder="全部服务商" onChange={event => setProvider(event.target.value)} /></label><label>筛选模型<input value={model} maxLength={128} placeholder="全部模型" onChange={event => setModel(event.target.value)} /></label></div>
      <div className="form-actions"><button type="submit" disabled={loading}>查询用量</button><button type="button" disabled={loading} onClick={() => setAttempt(value => value + 1)}>刷新用量</button></div>
    </form>
    {loading && <p className="field-hint">正在更新用量…</p>}
    {error && <p className="error-banner" role="alert">{error}</p>}
    {snapshot && <>
      {!snapshot.storage_available && <p className="availability-note">持久化不可用或历史读取不完整，统计可能缺失。</p>}
      <section className="runtime-summary" role="group" aria-label="用量汇总">
        <div className="runtime-section-heading"><h3>{snapshot.totals.calls.toLocaleString("zh-CN")} 次模型调用</h3><strong>{estimate(snapshot.totals)}</strong></div>
        <TokenTotals totals={snapshot.totals} />
        <p className="field-hint">仅累加已报告 tokens。缓存读取 / 写入包含在输入中，推理包含在输出中，请勿重复相加。</p>
        {(snapshot.totals.unpriced_calls > 0 || snapshot.totals.unknown_usage_calls > 0) && <p className="availability-note">{snapshot.totals.unpriced_calls} 次调用未计价；{snapshot.totals.unknown_usage_calls} 次调用未报告用量。汇总金额只覆盖可估算部分。</p>}
      </section>
      <h3 className="runtime-subheading">按模型汇总</h3>
      {snapshot.groups.length ? <ul className="runtime-group-list">{snapshot.groups.map(group => <li key={`${group.provider}\n${group.base_url}\n${group.model}`}>
        <div className="runtime-section-heading"><strong>{group.model}</strong><span>{group.totals.calls} 次 · {estimate(group.totals)}</span></div>
        <p className="field-hint">{group.provider} · {group.base_url}</p><TokenTotals totals={group.totals} />
      </li>)}</ul> : <p className="muted">此筛选范围内还没有调用记录。</p>}
      <details className="runtime-record-details"><summary>最近调用记录</summary>
        <p className="field-hint">最多展示最近 200 次调用。{snapshot.truncated ? "当前范围还有更多记录，可缩小日期范围查询。" : ""}</p>
        <ul className="runtime-call-list" aria-label="调用记录">{snapshot.records.map(record => <li key={record.id}>
          <div className="runtime-section-heading"><strong>{record.model}</strong><span className="runtime-badge">{statusLabels[record.status]}</span></div>
          <p className="field-hint">{new Date(record.started_at_ms).toLocaleString("zh-CN")} · {record.provider} · {(record.latency_ms / 1000).toFixed(2)} 秒{record.first_token_ms !== null ? ` · 首段 ${(record.first_token_ms / 1000).toFixed(2)} 秒` : ""}</p>
          <dl className="runtime-token-grid">{tokenFields.map(([field, label]) => <div key={field}><dt>{label}</dt><dd>{tokens(record.usage[field])}</dd></div>)}</dl>
          <p className="runtime-cost">{record.estimated_cost_microusd === null ? "未计价 / 未报告用量" : `估算 ${dollars(record.estimated_cost_microusd)}`}</p>
        </li>)}</ul>
        {!snapshot.records.length && <p className="muted">没有符合筛选条件的调用。</p>}
      </details>
    </>}
  </div>;
}
