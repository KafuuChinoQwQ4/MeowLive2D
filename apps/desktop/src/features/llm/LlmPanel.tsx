import { useEffect, useRef, useState, type FormEvent } from "react";
import type { LlmModelOption, LlmModelsRequest, LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot } from "@meowlive/contracts";
import { createLlmClient, reasoningEfforts, type LlmClient } from "../../services/server/llm";
import { useReasoningPreview } from "./useReasoningPreview";
import { LlmRuntimePanel } from "../llm-runtime/LlmRuntimePanel";
import type { LlmRuntimeClient } from "../../services/server/llm-runtime";
import { useFeedback } from "../../app/feedback/OperationFeedback";

type ApiFormat = "openai_responses" | "openai_chat" | "anthropic_messages" | "gemini_generate_content";
type LlmMode = "cloud" | "local";
type ProviderPreset = { value: string; label: string; apiFormat: ApiFormat; baseUrl: string };

export const providerPresets: ProviderPreset[] = [
  { value: "openai", label: "OpenAI / GPT", apiFormat: "openai_responses", baseUrl: "https://api.openai.com/v1" },
  { value: "claude", label: "Claude", apiFormat: "anthropic_messages", baseUrl: "https://api.anthropic.com/v1" },
  { value: "kimi", label: "Kimi", apiFormat: "openai_chat", baseUrl: "https://api.moonshot.cn/v1" },
  { value: "glm", label: "GLM", apiFormat: "openai_chat", baseUrl: "https://open.bigmodel.cn/api/paas/v4" },
  { value: "grok", label: "Grok", apiFormat: "openai_chat", baseUrl: "https://api.x.ai/v1" },
  { value: "gemini", label: "Gemini", apiFormat: "gemini_generate_content", baseUrl: "https://generativelanguage.googleapis.com/v1beta" },
  { value: "deepseek", label: "DeepSeek", apiFormat: "openai_chat", baseUrl: "https://api.deepseek.com" },
  { value: "custom", label: "自定义", apiFormat: "openai_chat", baseUrl: "" },
];
const formatLabels: Record<ApiFormat, string> = {
  openai_responses: "OpenAI Responses",
  openai_chat: "OpenAI Chat Completions",
  anthropic_messages: "Anthropic Messages",
  gemini_generate_content: "Gemini Generate Content",
};
const defaultClient = createLlmClient();

function identity(settings: LlmSettings) {
  let baseUrl = settings.base_url.trim().replace(/\/+$/u, "");
  try {
    const url = new URL(baseUrl);
    let path = url.pathname.replace(/\/+$/u, "");
    path = path.replace(/\/(?:chat\/completions|responses|messages|models\/[^/]+:generateContent|models)$/u, "");
    url.pathname = path || "/";
    baseUrl = url.toString();
  } catch { /* Validation reports incomplete or invalid URLs before any request. */ }
  return `${settings.provider}\n${settings.api_format}\n${baseUrl}`;
}

function inferConnection(baseUrl: string): Pick<LlmSettings, "provider" | "api_format"> {
  let provider = "custom";
  let apiFormat: ApiFormat = "openai_chat";
  try {
    const url = new URL(baseUrl.trim());
    const preset = providerPresets.find(item => item.baseUrl && new URL(item.baseUrl).hostname === url.hostname)
      ?? (url.hostname === "api.moonshot.ai" ? providerPresets.find(item => item.value === "kimi") : undefined);
    if (preset) { provider = preset.value; apiFormat = preset.apiFormat; }
    const path = url.pathname.replace(/\/+$/u, "");
    if (path.endsWith("/responses")) apiFormat = "openai_responses";
    else if (path.endsWith("/messages")) apiFormat = "anthropic_messages";
    else if (path.endsWith("/chat/completions")) apiFormat = "openai_chat";
    else if (/\/models\/[^/]+:generateContent$/u.test(path)) apiFormat = "gemini_generate_content";
  } catch { /* Infer again as the user completes the URL. */ }
  return { provider, api_format: apiFormat };
}

function validateConnection(settings: LlmSettings): string | null {
  const bytes = (value: string) => new TextEncoder().encode(value).length;
  if (!settings.provider.trim() || bytes(settings.provider) > 64) return "请选择有效的服务商。";
  if (!settings.base_url.trim()) return "请填写 API 地址。";
  if (bytes(settings.base_url) > 4096) return "API 地址不能超过 4096 个 UTF-8 字节。";
  try {
    const url = new URL(settings.base_url);
    if (!/^https?:$/u.test(url.protocol) || url.username || url.password || url.search || url.hash || /[\u0000-\u001f\u007f]/u.test(settings.base_url)) throw new Error();
  } catch { return "API 地址须为不含凭据、查询参数和片段的 HTTP(S) 地址。"; }
  return null;
}

function validate(settings: LlmSettings): string | null {
  const connectionError = validateConnection(settings);
  if (connectionError) return connectionError;
  if (!settings.model.trim() || new TextEncoder().encode(settings.model).length > 128) return "请先获取模型，再选择要使用的模型。";
  if (!Number.isInteger(settings.timeout_seconds) || settings.timeout_seconds < 1 || settings.timeout_seconds > 120) return "超时时间须为 1–120 秒。";
  if (!Number.isInteger(settings.max_tokens) || settings.max_tokens < 64 || settings.max_tokens > 65536) return "最大输出须为 64–65536 tokens。";
  if (settings.mode === "local" && settings.max_tokens > 1024) return "本地模式的最大输出不能超过 1024 tokens。";
  return null;
}

export function LlmPanel({ client = defaultClient, runtimeClient }: { client?: LlmClient; runtimeClient?: LlmRuntimeClient }) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<LlmSettingsSnapshot | null>(null);
  const [draft, setDraft] = useState<LlmSettings | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [clearApiKey, setClearApiKey] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState<"save" | "test" | "models" | null>(null);
  const [models, setModels] = useState<LlmModelOption[] | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const actions = useRef(new Set<AbortController>());
  const modelsAction = useRef<AbortController | null>(null);
  const modelsRequestId = useRef(0);
  const reasoning = useReasoningPreview(client, draft);
  const reasoningBlocked = Boolean(draft?.model && draft.reasoning_effort !== "default"
    && (reasoning.pending || reasoning.error || reasoning.result?.error));

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setPending(null); setError(""); setMessage(""); setModels(null);
    void client.getSettings(controller.signal).then(value => {
      if (controller.signal.aborted) return;
      setSnapshot(value); setDraft(value.settings); setApiKey(""); setClearApiKey(false);
      feedback.clearIssue("llm:configuration");
      if (loadAttempt > 0) feedback.success("LLM 配置已加载", "已读取保存的连接配置。");
    }).catch(reason => {
      if (controller.signal.aborted || (reason instanceof DOMException && reason.name === "AbortError")) return;
      setError(reason instanceof Error ? reason.message : "LLM 配置读取失败。");
      if (loadAttempt > 0) feedback.error("LLM 配置加载失败", reason);
      else feedback.reportIssue("llm:configuration", "LLM 配置读取失败", reason);
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [client, loadAttempt, feedback]);

  useEffect(() => () => { actions.current.forEach(controller => controller.abort()); }, [client]);

  const resetModels = () => {
    modelsAction.current?.abort();
    modelsAction.current = null;
    modelsRequestId.current += 1;
    setModels(null);
    setPending(current => current === "models" ? null : current);
  };
  const update = <K extends keyof LlmSettings>(key: K, value: LlmSettings[K]) => {
    const connectionChanged = key === "base_url" || key === "api_format" || key === "mode";
    if (connectionChanged) resetModels();
    setDraft(current => current ? {
      ...current, [key]: value,
      ...(connectionChanged ? { model: "" } : {}),
      ...(key === "base_url" ? inferConnection(String(value)) : {}),
    } : current);
    setError(""); setMessage("");
  };
  const changeProvider = (provider: string) => {
    const preset = providerPresets.find(item => item.value === provider)!;
    resetModels();
    setDraft(current => current ? { ...current, provider, api_format: preset.apiFormat, base_url: preset.baseUrl, model: "" } : current);
    setError(""); setMessage("");
  };
  const checkRequest = (requireModel: boolean): boolean => {
    if (!draft || !snapshot) return false;
    const validation = requireModel ? validate(draft) : validateConnection(draft);
    if (validation) { setError(validation); feedback.error("LLM 配置检查失败", validation); return false; }
    if (identity(draft) !== identity(snapshot.settings) && snapshot.key_configured && !apiKey && !clearApiKey) {
      const message = "连接身份已变化，请填写新的 API 密钥或明确移除已保存密钥。";
      setError(message); feedback.error("LLM 配置检查失败", message); return false;
    }
    return true;
  };
  const buildRequest = (): LlmSettingsRequest | null => {
    if (!draft || !snapshot) return null;
    if (!checkRequest(true)) return null;
    if (reasoningBlocked) return null;
    return { settings: { ...draft, provider: draft.provider.trim(), base_url: draft.base_url.trim(), model: draft.model.trim() }, api_key: apiKey || null, clear_api_key: clearApiKey };
  };
  const fetchModels = async () => {
    if (pending || !draft || !checkRequest(false)) return;
    const request: LlmModelsRequest = {
      provider: draft.provider.trim(), api_format: draft.api_format, base_url: draft.base_url.trim(),
      mode: draft.mode, api_key: apiKey || null, clear_api_key: clearApiKey,
    };
    const selected = draft.model;
    const controller = new AbortController();
    const requestId = ++modelsRequestId.current;
    modelsAction.current = controller; actions.current.add(controller);
    setPending("models"); setError(""); setMessage(""); setModels([]);
    setDraft(current => current ? { ...current, model: "" } : current);
    try {
      const result = await client.listModels(request, controller.signal);
      if (controller.signal.aborted || requestId !== modelsRequestId.current) return;
      setModels(result.models);
      setDraft(current => current ? { ...current, base_url: result.base_url, model: result.models.some(model => model.id === selected) ? selected : "" } : current);
    } catch (reason) {
      if (!controller.signal.aborted && requestId === modelsRequestId.current && !(reason instanceof DOMException && reason.name === "AbortError")) {
        setModels(null);
        setError(reason instanceof Error ? reason.message : "获取模型失败，请检查 API 地址和密钥后重试。");
        feedback.error("获取 LLM 模型失败", reason);
      }
    } finally {
      actions.current.delete(controller);
      if (requestId === modelsRequestId.current && !controller.signal.aborted) { modelsAction.current = null; setPending(null); }
    }
  };
  const run = async (kind: "save" | "test") => {
    if (pending) return;
    if (kind === "save" && snapshot?.storage_available === false) return;
    const request = buildRequest();
    if (!request) return;
    const controller = new AbortController(); actions.current.add(controller);
    setPending(kind); setError(""); setMessage("");
    try {
      if (kind === "save") {
        const value = await client.saveSettings(request, controller.signal);
        if (controller.signal.aborted) return;
        setSnapshot(value); setDraft(value.settings); setApiKey(""); setClearApiKey(false);
        setMessage(value.restart_required ? "配置已保存。重启主服务后，新配置才会生效。" : "配置已保存并已生效。");
        feedback.success("LLM 配置已保存", value.restart_required ? "重启主服务后，新配置才会生效。" : "配置已保存并已生效。");
      } else {
        const result = await client.testSettings(request, controller.signal);
        if (controller.signal.aborted) return;
        setMessage(result.message);
        feedback.success("LLM 连接测试成功", result.message);
      }
    } catch (reason) {
      if (!controller.signal.aborted && !(reason instanceof DOMException && reason.name === "AbortError")) { setError(reason instanceof Error ? reason.message : "LLM 操作失败。"); feedback.error(kind === "save" ? "保存 LLM 配置失败" : "LLM 连接测试失败", reason); }
    } finally {
      actions.current.delete(controller);
      if (!controller.signal.aborted) setPending(null);
    }
  };
  const submit = (event: FormEvent) => { event.preventDefault(); void run("save"); };
  const disabled = loading || pending === "save" || pending === "test" || !draft;
  const actionDisabled = disabled || pending === "models";
  const savedKeyAvailable = snapshot?.key_configured && draft && identity(draft) === identity(snapshot.settings);

  return <div className="live-workspace" aria-labelledby="llm-heading">
    <section className="connection-card">
      <div><h2 id="llm-heading">LLM 接入配置</h2><p className="server-address">{client.baseUrl}</p></div>
      <span className={`connection-pill ${snapshot?.active_model ? "connected" : "disconnected"}`}>{snapshot?.active_model || (loading ? "正在读取" : "尚未配置")}</span>
    </section>
    {error && <div className="error-banner" role="alert">{error}{!snapshot && !loading && <div className="form-actions"><button type="button" onClick={() => setLoadAttempt(value => value + 1)}>重新加载</button></div>}</div>}
    {(message || snapshot?.restart_required) && <p className="success-banner" role="status">{message || "配置已保存但尚未生效，请重启主服务。"} {snapshot?.restart_required && <a href="#overview">前往运行总览</a>}</p>}
    {snapshot && !snapshot.storage_available && <p className="availability-note">此主服务未启用配置保存，可测试连接；请使用项目主服务启动入口后保存。</p>}
    <section className="panel">
      <form onSubmit={submit} noValidate>
        <div className="workspace-columns">
          <div>
            <label htmlFor="llm-base-url">API 地址</label>
            <input id="llm-base-url" value={draft?.base_url ?? ""} disabled={disabled} autoComplete="url" placeholder="https://example.com/v1" onChange={event => update("base_url", event.target.value)} />
            <p className="field-hint">填写供应商地址或完整端点，将自动识别常见服务的 API 格式。</p>
            <label htmlFor="llm-key">API 密钥</label>
            <input id="llm-key" type="password" value={apiKey} disabled={disabled || clearApiKey} autoComplete="new-password" placeholder={savedKeyAvailable ? "已安全保存；留空则继续使用" : "输入 API 密钥（如服务需要）"} onChange={event => { resetModels(); update("model", ""); setApiKey(event.target.value); setClearApiKey(false); }} />
            {snapshot?.key_configured && <label className="checkbox-field"><input type="checkbox" checked={clearApiKey} disabled={disabled} onChange={event => { resetModels(); update("model", ""); setClearApiKey(event.target.checked); if (event.target.checked) setApiKey(""); }} /><span>移除已保存密钥</span></label>}
          </div>
          <div>
            <div className="form-actions"><button type="button" disabled={actionDisabled} onClick={() => { void fetchModels(); }}>{pending === "models" ? "正在获取模型…" : "获取模型"}</button></div>
            <label htmlFor="llm-model">模型</label>
            <select id="llm-model" value={draft?.model ?? ""} disabled={actionDisabled} onChange={event => update("model", event.target.value)} aria-describedby="llm-model-help">
              <option value="">{models?.length ? "请选择模型" : "请先获取模型"}</option>
              {models === null && draft?.model && <option value={draft.model}>{draft.model}（已保存，尚未验证）</option>}
              {models?.map(model => <option key={model.id} value={model.id}>{model.name === model.id ? model.id : `${model.name}（${model.id}）`}</option>)}
            </select>
            <div id="llm-model-help" aria-live="polite">
              {pending === "models" ? <p className="field-hint">正在读取供应商可用模型…</p>
                : models !== null ? <><p className="field-hint">已获取 {models.length} 个模型。</p>{models.length === 0 && <p className="field-hint">供应商未返回可选模型，请检查地址、密钥权限或高级设置后重试。</p>}</>
                  : <p className="field-hint">填写地址和密钥后获取模型，再从列表选择。获取模型不会保存配置。</p>}
            </div>
            <label htmlFor="llm-reasoning">推理强度</label>
            <select id="llm-reasoning" value={draft?.reasoning_effort ?? "default"} disabled={actionDisabled || !draft?.model} onChange={event => update("reasoning_effort", event.target.value)} aria-describedby="llm-reasoning-help">
              {reasoningEfforts.map(effort => <option key={effort} value={effort}>{effort === "default" ? "default（模型默认）" : effort}</option>)}
            </select>
            <div id="llm-reasoning-help" aria-live="polite" aria-busy={reasoning.pending}>
              <p className="field-hint">default 保留模型默认行为；其余档位按当前模型能力映射。保存后重启主服务生效。</p>
              {!draft?.model ? <p className="field-hint">选择模型后可查看实际生效档位。</p>
                : reasoning.pending ? <p className="field-hint">正在确认模型推理能力…</p>
                  : reasoning.result && <>
                    <p className="field-hint">实际生效：{reasoning.result.effective && reasoning.result.effective !== "default" ? reasoning.result.effective : "模型默认（不覆盖）"}</p>
                    {reasoning.result.supported.length > 0 && <p className="field-hint">支持档位：{reasoning.result.supported.join(" / ")}</p>}
                    {reasoning.result.budget_tokens !== null && <p className="field-hint">推理预算：{reasoning.result.budget_tokens} tokens（包含在最大输出上限内）</p>}
                    {reasoning.result.note && <p className="field-hint">{reasoning.result.note}</p>}
                    {reasoning.result.error && <p className="field-error" role="alert">{reasoning.result.error}</p>}
                  </>}
              {reasoning.error && <><p className="field-error" role="alert">{reasoning.error}</p><button type="button" disabled={actionDisabled} onClick={reasoning.retry}>重新预览</button></>}
            </div>
          </div>
        </div>
        <details>
          <summary>高级设置</summary>
          <div className="workspace-columns">
            <div>
              <label htmlFor="llm-provider">服务商</label>
              <select id="llm-provider" value={draft?.provider ?? "custom"} disabled={disabled} onChange={event => changeProvider(event.target.value)}>
                {!providerPresets.some(item => item.value === draft?.provider) && draft && <option value={draft.provider}>{draft.provider}</option>}
                {providerPresets.map(item => <option key={item.value} value={item.value}>{item.label}</option>)}
              </select>
              <label htmlFor="llm-format">API 格式</label>
              <select id="llm-format" value={draft?.api_format ?? "openai_chat"} disabled={disabled} onChange={event => update("api_format", event.target.value as ApiFormat)}>
                {(Object.keys(formatLabels) as ApiFormat[]).map(value => <option key={value} value={value}>{formatLabels[value]}</option>)}
              </select>
              <p className="field-hint">兼容网关可手动调整 API 格式；再次修改地址时会重新识别。</p>
            </div>
            <div>
              <label htmlFor="llm-mode">运行模式</label>
              <select id="llm-mode" value={draft?.mode ?? "cloud"} disabled={disabled} onChange={event => update("mode", event.target.value as LlmMode)}><option value="cloud">云端</option><option value="local">本地</option></select>
              <label htmlFor="llm-timeout">超时时间（秒）</label>
              <input id="llm-timeout" type="number" min={1} max={120} step={1} value={draft?.timeout_seconds ?? 30} disabled={disabled} onChange={event => update("timeout_seconds", Number(event.target.value))} />
              <label htmlFor="llm-max-tokens">最大输出（tokens）</label>
              <input id="llm-max-tokens" type="number" min={64} max={draft?.mode === "local" ? 1024 : 65536} step={64} value={draft?.max_tokens ?? 1024} disabled={disabled} onChange={event => update("max_tokens", Number(event.target.value))} />
              <label className="checkbox-field"><input type="checkbox" checked={draft?.json_mode ?? false} disabled={disabled} onChange={event => update("json_mode", event.target.checked)} /><span>要求 JSON 格式输出</span></label>
            </div>
          </div>
        </details>
        <p className="availability-note">测试可能产生少量调用费用，不会自动保存。</p>
        <div className="form-actions">
          <button type="submit" className="primary-button" disabled={actionDisabled || reasoningBlocked || snapshot?.storage_available === false}>{pending === "save" ? "正在保存…" : "保存配置"}</button>
          <button type="button" disabled={actionDisabled || reasoningBlocked} onClick={() => { void run("test"); }}>{pending === "test" ? "正在测试…" : "测试连接"}</button>
        </div>
      </form>
    </section>
    {runtimeClient && <LlmRuntimePanel client={runtimeClient} connection={snapshot?.settings} />}
  </div>;
}
