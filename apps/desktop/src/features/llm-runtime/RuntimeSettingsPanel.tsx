import { useEffect, useRef, useState, type FormEvent } from "react";
import type { AgentRuntimeSettings, AgentRuntimeSettingsSnapshot, LlmPrice } from "@meowlive/contracts";
import { validRuntimeSearchEndpoint, type LlmRuntimeClient } from "../../services/server/llm-runtime";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export type LlmConnectionIdentity = Pick<LlmPrice, "provider" | "base_url" | "model">;
const priceFields = [
  ["input_usd_per_million", "输入单价"], ["output_usd_per_million", "输出单价"],
  ["cache_read_usd_per_million", "缓存读取单价"], ["cache_write_usd_per_million", "缓存写入单价"],
] as const;
const toggles = [["cache_enabled", "自动缓存优化"], ["streaming", "流式接收"], ["tools_enabled", "允许工具调用"],
  ["environment_enabled", "附带当前环境"], ["web_search_enabled", "允许网页搜索"]] as const;
const searchIdentity = (settings: AgentRuntimeSettings) => `${settings.search_provider}\n${settings.search_endpoint.trim()}`;
function validUrl(value: string) {
  try { const url = new URL(value); return /^https?:$/u.test(url.protocol) && !url.username && !url.password && !url.search && !url.hash && !/[\u0000-\u001f\u007f]/u.test(value); }
  catch { return false; }
}

export function RuntimeSettingsPanel({ client, connection, active, onSaved }: { client: LlmRuntimeClient; active: boolean; connection?: LlmConnectionIdentity; onSaved: () => void }) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<AgentRuntimeSettingsSnapshot | null>(null);
  const [draft, setDraft] = useState<AgentRuntimeSettings | null>(null);
  const [key, setKey] = useState("");
  const [clearKey, setClearKey] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [attempt, setAttempt] = useState(0);
  const action = useRef<AbortController | null>(null);
  useEffect(() => {
    if (!active || snapshot) return;
    const controller = new AbortController();
    setError("");
    void client.getSettings(controller.signal).then(value => {
      if (!controller.signal.aborted) { setSnapshot(value); setDraft(value.settings); }
    }).catch(reason => { if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : "运行配置读取失败。"); });
    return () => controller.abort();
  }, [client, attempt, active, snapshot]);
  useEffect(() => { setSnapshot(null); setDraft(null); setKey(""); setClearKey(false); setMessage(""); setPending(false); }, [client]);
  useEffect(() => { if (!active) { action.current?.abort(); setPending(false); } }, [active]);
  useEffect(() => () => action.current?.abort(), [client]);
  const update = <K extends keyof AgentRuntimeSettings>(field: K, value: AgentRuntimeSettings[K]) => {
    setDraft(current => current ? { ...current, [field]: value } : null); setError(""); setMessage("");
  };
  const updatePrice = (index: number, patch: Partial<LlmPrice>) => {
    if (draft) update("prices", draft.prices.map((price, i) => i === index ? { ...price, ...patch } : price));
  };
  const addPrice = () => {
    if (draft && connection) update("prices", [...draft.prices, { provider: connection.provider, base_url: connection.base_url, model: connection.model, input_usd_per_million: NaN, output_usd_per_million: NaN, cache_read_usd_per_million: NaN, cache_write_usd_per_million: NaN }]);
  };
  const fail = (message: string) => { setError(message); feedback.error("运行配置检查失败", message); };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!draft || !snapshot || pending || !snapshot.storage_available) return;
    if (snapshot.search_key_configured && searchIdentity(draft) !== searchIdentity(snapshot.settings) && !key.trim() && !clearKey) {
      fail("搜索连接已变化，请填写新的搜索密钥或明确移除已保存密钥。"); return;
    }
    if (!validRuntimeSearchEndpoint(draft.search_provider, draft.search_endpoint.trim())) { fail("请填写不含凭据和查询参数的 HTTPS 搜索地址（最多 2048 字节）；本地 SearXNG 可使用 HTTP 回环地址。"); return; }
    if (draft.tools_enabled && draft.web_search_enabled && (!draft.search_endpoint || (draft.search_provider === "brave" && !(key.trim() || snapshot.search_key_configured && !clearKey && searchIdentity(draft) === searchIdentity(snapshot.settings))))) {
      fail("启用网页搜索需要配置 Brave 地址和密钥，或选择可用的 SearXNG 地址。"); return;
    }
    if (!Number.isInteger(draft.max_tool_rounds) || draft.max_tool_rounds < 1 || draft.max_tool_rounds > 3 || !Number.isInteger(draft.tool_timeout_seconds) || draft.tool_timeout_seconds < 1 || draft.tool_timeout_seconds > 8) {
      fail("工具轮次须为 1–3，单次工具超时须为 1–8 秒。"); return;
    }
    const identities = new Set<string>();
    for (const price of draft.prices) {
      if (!price.provider.trim() || new TextEncoder().encode(price.provider).length > 64 || !price.model.trim() || new TextEncoder().encode(price.model).length > 128 || !validUrl(price.base_url)
        || priceFields.some(([field]) => !Number.isFinite(price[field]) || price[field] < 0 || price[field] > 1_000_000)) {
        fail("请填写有效的价格身份和四项单价；单价须为 0–1,000,000 USD / 百万 tokens。"); return;
      }
      const identity = `${price.provider.trim()}\n${price.base_url.trim().replace(/\/+$/u, "")}\n${price.model.trim()}`;
      if (identities.has(identity)) { fail("同一服务商、地址和模型只能配置一条价格。"); return; }
      identities.add(identity);
    }
    const controller = new AbortController(); action.current = controller;
    setPending(true); setError(""); setMessage("");
    try {
      const next = await client.saveSettings({ settings: { ...draft, search_endpoint: draft.search_endpoint.trim(), prices: draft.prices.map(price => ({ ...price, provider: price.provider.trim(), base_url: price.base_url.trim(), model: price.model.trim() })) }, search_api_key: key.trim() || null, clear_search_api_key: clearKey }, controller.signal);
      if (controller.signal.aborted) return;
      setSnapshot(next); setDraft(next.settings); setKey(""); setClearKey(false); setMessage("运行配置已保存并生效。");
      feedback.success("运行配置已保存", "运行能力、搜索与价格配置已生效。"); onSaved();
    } catch (reason) {
      if (!controller.signal.aborted) { const message = reason instanceof Error ? reason.message : "运行配置保存失败。"; setError(message); feedback.error("运行配置保存失败", message); }
    } finally { if (!controller.signal.aborted) setPending(false); }
  };
  if (!draft || !snapshot) return error ? <div className="error-banner" role="alert">{error}<button type="button" onClick={() => setAttempt(value => value + 1)}>重新读取运行配置</button></div> : <p className="muted">正在读取运行配置…</p>;
  const sameSearch = searchIdentity(draft) === searchIdentity(snapshot.settings);
  return <form className="runtime-settings" onSubmit={event => { void submit(event); }} noValidate>
    {error && <p className="error-banner" role="alert">{error}</p>}
    {message && <p className="success-banner" role="status">{message}</p>}
    {!snapshot.storage_available && <p className="availability-note">当前主服务的持久保存不可用，运行配置暂不能保存。</p>}
    <fieldset disabled={pending} className="runtime-fieldset">
      <div className="runtime-toggle-grid">{toggles.map(([field, label]) => <label key={field} className="checkbox-field"><input type="checkbox" checked={draft[field]} onChange={event => update(field, event.target.checked)} /><span>{label}</span></label>)}</div>
      <p className="field-hint">缓存命中由供应商决定；兼容接口不支持时可关闭缓存优化或流式接收。环境读取仅包括时间、直播、播放和 OBS 状态。</p>
      {!draft.tools_enabled && <p className="availability-note">模型主动查询工具已关闭；环境快照仍由“附带当前环境”开关控制。</p>}
      <div className="runtime-fields">
        <label>最多工具轮次<input type="number" min={1} max={3} value={draft.max_tool_rounds} onChange={event => update("max_tool_rounds", Number(event.target.value))} /></label>
        <label>单次工具超时（秒）<input type="number" min={1} max={8} value={draft.tool_timeout_seconds} onChange={event => update("tool_timeout_seconds", Number(event.target.value))} /></label>
      </div>
      <div className="runtime-fields">
        <label>搜索服务<select value={draft.search_provider} onChange={event => { update("search_provider", event.target.value); update("search_endpoint", event.target.value === "brave" ? "https://api.search.brave.com/res/v1/web/search" : ""); }}><option value="brave">Brave Search</option><option value="searxng">SearXNG</option></select></label>
        <label>搜索地址<input type="url" value={draft.search_endpoint} maxLength={2048} placeholder={draft.search_provider === "searxng" ? "http://127.0.0.1:8080/search" : "https://api.search.brave.com/res/v1/web/search"} onChange={event => update("search_endpoint", event.target.value)} /></label>
      </div>
      <label>搜索 API 密钥<input type="password" value={key} maxLength={4096} autoComplete="new-password" disabled={clearKey} placeholder={snapshot.search_key_configured && sameSearch ? "已保存；同一连接留空则保留" : "输入新的搜索密钥"} onChange={event => { setKey(event.target.value); setMessage(""); }} /></label>
      {snapshot.search_key_configured && <label className="checkbox-field"><input type="checkbox" checked={clearKey} onChange={event => { setClearKey(event.target.checked); if (event.target.checked) setKey(""); }} /><span>移除已保存搜索密钥</span></label>}
      <p className="field-hint">搜索需要 Brave API 密钥，或支持 JSON 查询的 SearXNG。密钥不会回显；地址或服务变化后须重新填写或明确移除。</p>
      <details className="runtime-price-details"><summary>模型价格</summary>
        <p className="muted">单位：USD / 百万 tokens。价格按服务商、API 地址和模型精确匹配；请填写全部四项，仅免费部分填写 0。费用是本地估算，实际账单以供应商为准。</p>
        <p className="field-hint">未单独计费的缓存写入请填写普通输入单价；不清楚缓存读取折扣时，也可填写普通输入单价，按不打折估算。供应商未报告缓存明细且单价不同时，该次费用会显示为未计价。</p>
        {draft.prices.map((price, index) => <fieldset className="runtime-price-row" key={index}><legend>价格 {index + 1}</legend>
          <div className="runtime-fields"><label>服务商<input aria-label={`价格服务商 ${index + 1}`} value={price.provider} maxLength={64} onChange={event => updatePrice(index, { provider: event.target.value })} /></label><label>模型<input aria-label={`价格模型 ${index + 1}`} value={price.model} maxLength={128} onChange={event => updatePrice(index, { model: event.target.value })} /></label></div>
          <label>API 地址<input aria-label={`价格 API 地址 ${index + 1}`} value={price.base_url} maxLength={4096} onChange={event => updatePrice(index, { base_url: event.target.value })} /></label>
          <div className="runtime-fields">{priceFields.map(([field, label]) => <label key={field}>{label}<input aria-label={`${label} ${index + 1}`} type="number" min={0} max={1000000} step="any" value={Number.isFinite(price[field]) ? price[field] : ""} onChange={event => updatePrice(index, { [field]: event.target.value === "" ? NaN : Number(event.target.value) })} /></label>)}</div>
          <button type="button" className="runtime-remove" onClick={() => update("prices", draft.prices.filter((_, i) => i !== index))}>移除价格 {index + 1}</button>
        </fieldset>)}
        {!draft.prices.length && <p className="muted">尚未配置价格，用量会保留，费用显示为未计价。</p>}
        <div className="form-actions"><button type="button" disabled={!connection?.model || draft.prices.length >= 64} onClick={addPrice}>添加当前连接价格</button></div>
        {!connection?.model && <p className="field-hint">先保存 LLM 连接并选择模型，再添加价格。</p>}
      </details>
      <div className="form-actions"><button className="primary-button" type="submit" disabled={!snapshot.storage_available}>{pending ? "正在保存…" : "保存运行配置"}</button></div>
    </fieldset>
  </form>;
}
