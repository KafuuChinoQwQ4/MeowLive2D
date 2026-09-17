import { useEffect, useRef, useState, type FormEvent } from "react";
import type { LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot } from "@meowlive/contracts";
import { createLlmClient, type LlmClient } from "../../services/server/llm";
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
  return `${settings.provider}\n${settings.api_format}\n${settings.base_url.replace(/\/+$/u, "")}`;
}

function validate(settings: LlmSettings): string | null {
  const bytes = (value: string) => new TextEncoder().encode(value).length;
  if (!settings.provider.trim() || bytes(settings.provider) > 64) return "请选择有效的服务商。";
  if (!settings.base_url.trim()) return "请填写 API 地址。";
  if (bytes(settings.base_url) > 4096) return "API 地址不能超过 4096 个 UTF-8 字节。";
  try {
    const url = new URL(settings.base_url);
    if (!/^https?:$/u.test(url.protocol) || url.username || url.password || url.search || url.hash) throw new Error();
  } catch { return "API 地址须为不含凭据、查询参数和片段的 HTTP(S) 地址。"; }
  if (!settings.model.trim() || bytes(settings.model) > 128) return "请填写不超过 128 个 UTF-8 字节的模型名称。";
  if (!Number.isInteger(settings.timeout_seconds) || settings.timeout_seconds < 1 || settings.timeout_seconds > 120) return "超时时间须为 1–120 秒。";
  if (!Number.isInteger(settings.max_tokens) || settings.max_tokens < 64 || settings.max_tokens > 4096) return "最大输出须为 64–4096 tokens。";
  if (settings.mode === "local" && settings.max_tokens > 1024) return "本地模式的最大输出不能超过 1024 tokens。";
  return null;
}

export function LlmPanel({ client = defaultClient }: { client?: LlmClient }) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<LlmSettingsSnapshot | null>(null);
  const [draft, setDraft] = useState<LlmSettings | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [clearApiKey, setClearApiKey] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState<"save" | "test" | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const actions = useRef(new Set<AbortController>());

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setPending(null); setError(""); setMessage("");
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

  const update = <K extends keyof LlmSettings>(key: K, value: LlmSettings[K]) => {
    setDraft(current => current ? { ...current, [key]: value } : current);
    setError(""); setMessage("");
  };
  const changeProvider = (provider: string) => {
    const preset = providerPresets.find(item => item.value === provider)!;
    setDraft(current => current ? { ...current, provider, api_format: preset.apiFormat, base_url: preset.baseUrl, model: "" } : current);
    setApiKey(""); setClearApiKey(false); setError(""); setMessage("");
  };
  const buildRequest = (): LlmSettingsRequest | null => {
    if (!draft || !snapshot) return null;
    const validation = validate(draft);
    if (validation) { setError(validation); feedback.error("LLM 配置检查失败", validation); return null; }
    const changedIdentity = identity(draft) !== identity(snapshot.settings);
    if (changedIdentity && snapshot.key_configured && !apiKey && !clearApiKey) {
      setError("连接身份已变化，请填写新的 API 密钥或明确移除已保存密钥。"); feedback.error("LLM 配置检查失败", "连接身份已变化，请填写新的 API 密钥或明确移除已保存密钥。"); return null;
    }
    return { settings: { ...draft, provider: draft.provider.trim(), base_url: draft.base_url.trim(), model: draft.model.trim() }, api_key: apiKey || null, clear_api_key: clearApiKey };
  };
  const run = async (kind: "save" | "test") => {
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
  const disabled = loading || pending !== null || !draft;

  return <div className="live-workspace" aria-labelledby="llm-heading">
    <section className="connection-card">
      <div><p className="eyebrow">模型连接</p><h2 id="llm-heading">LLM 接入配置</h2><p className="server-address">{client.baseUrl}</p></div>
      <span className={`connection-pill ${snapshot?.active_model ? "connected" : "disconnected"}`}>{snapshot?.active_model || (loading ? "正在读取" : "尚未配置")}</span>
    </section>
    {error && <div className="error-banner" role="alert">{error}{!snapshot && !loading && <div className="form-actions"><button type="button" onClick={() => setLoadAttempt(value => value + 1)}>重新加载</button></div>}</div>}
    {(message || snapshot?.restart_required) && <p className="success-banner" role="status">{message || "配置已保存但尚未生效，请重启主服务。"} {snapshot?.restart_required && <a href="#overview">前往运行总览</a>}</p>}
    {snapshot && !snapshot.storage_available && <p className="availability-note">此主服务未启用配置保存，可测试连接；请使用项目主服务启动入口后保存。</p>}
    <section className="panel">
      <form onSubmit={submit} noValidate>
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
            <label htmlFor="llm-base-url">API 地址</label>
            <input id="llm-base-url" value={draft?.base_url ?? ""} disabled={disabled} autoComplete="url" placeholder="https://example.com/v1" onChange={event => update("base_url", event.target.value)} />
            <p className="field-hint">可填写 API 根地址或完整端点，主服务会按所选 API 格式规范化。</p>
            <label htmlFor="llm-model">模型名称</label>
            <input id="llm-model" value={draft?.model ?? ""} disabled={disabled} autoComplete="off" maxLength={128} placeholder="输入供应商提供的模型名称" onChange={event => update("model", event.target.value)} />
            <p className="field-hint">模型名称最多 128 个 UTF-8 字节。</p>
            <label htmlFor="llm-key">API 密钥</label>
            <input id="llm-key" type="password" value={apiKey} disabled={disabled || clearApiKey} autoComplete="new-password" placeholder={snapshot?.key_configured ? "已安全保存；留空则继续使用" : "输入 API 密钥（如服务需要）"} onChange={event => { setApiKey(event.target.value); setClearApiKey(false); setError(""); setMessage(""); }} />
            <label className="checkbox-field"><input type="checkbox" checked={clearApiKey} disabled={disabled || !snapshot?.key_configured} onChange={event => { setClearApiKey(event.target.checked); if (event.target.checked) setApiKey(""); setError(""); setMessage(""); }} /><span>移除已保存密钥</span></label>
          </div>
          <div>
            <label htmlFor="llm-mode">运行模式</label>
            <select id="llm-mode" value={draft?.mode ?? "cloud"} disabled={disabled} onChange={event => update("mode", event.target.value as LlmMode)}><option value="cloud">云端</option><option value="local">本地</option></select>
            <label htmlFor="llm-timeout">超时时间（秒）</label>
            <input id="llm-timeout" type="number" min={1} max={120} step={1} value={draft?.timeout_seconds ?? 30} disabled={disabled} onChange={event => update("timeout_seconds", Number(event.target.value))} />
            <label htmlFor="llm-max-tokens">最大输出（tokens）</label>
            <input id="llm-max-tokens" type="number" min={64} max={draft?.mode === "local" ? 1024 : 4096} step={64} value={draft?.max_tokens ?? 1024} disabled={disabled} onChange={event => update("max_tokens", Number(event.target.value))} />
            <label className="checkbox-field"><input type="checkbox" checked={draft?.json_mode ?? false} disabled={disabled} onChange={event => update("json_mode", event.target.checked)} /><span>要求 JSON 格式输出</span></label>
            <p className="availability-note">测试连接会使用当前草稿发送一个通用预设请求，可能产生少量调用费用。测试不会自动保存配置。</p>
          </div>
        </div>
        <div className="form-actions">
          <button type="submit" className="primary-button" disabled={disabled || snapshot?.storage_available === false}>{pending === "save" ? "正在保存…" : "保存配置"}</button>
          <button type="button" disabled={disabled} onClick={() => { void run("test"); }}>{pending === "test" ? "正在测试…" : "测试连接"}</button>
        </div>
      </form>
    </section>
  </div>;
}
