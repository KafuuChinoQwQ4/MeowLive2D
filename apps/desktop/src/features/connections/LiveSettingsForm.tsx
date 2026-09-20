import { useEffect, useRef, useState, type FormEvent } from "react";
import type { LiveConnectionPhase, LiveSettingsRequest, LiveSettingsSnapshot } from "@meowlive/contracts";
import { useFeedback } from "../../app/feedback/OperationFeedback";
import type { LiveClient } from "../../services/server/live";

const credentialFields = [
  { key: "access_key_id", label: "AccessKey ID", maxBytes: 256, help: "复制直播开放平台提供的 AccessKey ID。" },
  { key: "access_key_secret", label: "AccessKey Secret", maxBytes: 512, help: "与 AccessKey ID 配套的密钥，请完整复制。" },
  { key: "identity_code", label: "主播身份码", maxBytes: 512, help: "由本次接入的主播提供，用于授权连接主播的直播间。" },
] as const;
const emptyCredentials = { access_key_id: "", access_key_secret: "", identity_code: "" };
type Credentials = typeof emptyCredentials;

function validAppId(value: string): boolean {
  return /^\d{1,19}$/u.test(value) && BigInt(value) <= 9223372036854775807n;
}

function appIdentity(value: string): string {
  const trimmed = value.trim();
  return validAppId(trimmed) ? BigInt(trimmed).toString() : trimmed;
}

export function LiveSettingsForm({ client, phase, actionPending, onSaved, onSavingChange }: {
  client: LiveClient;
  phase?: LiveConnectionPhase;
  actionPending: boolean;
  onSaved: () => Promise<void>;
  onSavingChange: (saving: boolean) => void;
}) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<LiveSettingsSnapshot | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [appId, setAppId] = useState("");
  const [credentials, setCredentials] = useState<Credentials>(emptyCredentials);
  const [clearCredentials, setClearCredentials] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const saveController = useRef<AbortController | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setSaving(false); setSnapshot(null); setError(""); setMessage("");
    setCredentials(emptyCredentials); setClearCredentials(false);
    void client.getSettings(controller.signal).then(value => {
      if (controller.signal.aborted) return;
      setSnapshot(value); setEnabled(value.enabled); setAppId(value.app_id);
      feedback.clearIssue("live:configuration");
    }).catch(reason => {
      if (controller.signal.aborted) return;
      setError(reason instanceof Error ? reason.message : "直播配置读取失败，请重试。");
      feedback.reportIssue("live:configuration", "直播配置读取失败", reason);
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [client, loadAttempt, feedback]);

  useEffect(() => () => { saveController.current?.abort(); saveController.current = null; onSavingChange(false); }, [client, onSavingChange]);

  const busyConnection = Boolean(phase && ["connecting", "connected", "reconnecting", "disconnecting"].includes(phase));
  const unavailable = !snapshot || loading || saving || !snapshot.storage_available || busyConnection || actionPending || !phase;
  const appChanged = snapshot !== null && appIdentity(appId) !== appIdentity(snapshot.app_id);
  const clearNotice = () => { setError(""); setMessage(""); };

  function buildRequest(): LiveSettingsRequest | string {
    const trimmedAppId = appId.trim();
    if ((trimmedAppId && !validAppId(trimmedAppId)) || (enabled && (!trimmedAppId || BigInt(trimmedAppId) === 0n))) {
      return "应用 ID 须为 1–9223372036854775807 的整数，请完整复制平台提供的数字。";
    }
    const request: LiveSettingsRequest = {
      enabled, app_id: trimmedAppId ? BigInt(trimmedAppId).toString() : "", clear_credentials: clearCredentials,
      access_key_id: null, access_key_secret: null, identity_code: null,
    };
    if (clearCredentials) return request;
    for (const field of credentialFields) {
      const value = credentials[field.key].trim();
      if (new TextEncoder().encode(value).length > field.maxBytes || /[\u0000-\u001f\u007f-\u009f]/u.test(value)) {
        return `${field.label} 不能包含控制字符，长度不能超过 ${field.maxBytes} 个 UTF-8 字节。`;
      }
      request[field.key] = value || null;
    }
    const savedCredentials = credentialFields.some(field => snapshot?.[`${field.key}_configured`]);
    if (appChanged && savedCredentials && credentialFields.some(field => !request[field.key])) {
      return "应用 ID 已变更，请重新填写全部三项凭据，或移除已保存的全部直播凭据。";
    }
    if (enabled) {
      const missing = credentialFields.filter(field => !request[field.key] && !snapshot?.[`${field.key}_configured`]);
      if (missing.length) return `启用前请填写${missing.map(field => field.label).join("、")}。`;
    }
    return request;
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    if (unavailable || saveController.current) return;
    const request = buildRequest();
    if (typeof request === "string") { setError(request); feedback.error("直播配置检查失败", request); return; }
    const controller = new AbortController();
    saveController.current = controller;
    setSaving(true); onSavingChange(true); clearNotice();
    try {
      const value = await client.saveSettings(request, controller.signal);
      if (controller.signal.aborted) return;
      setSnapshot(value); setEnabled(value.enabled); setAppId(value.app_id);
      setCredentials(emptyCredentials); setClearCredentials(false);
      const notice = value.enabled ? "配置已保存并生效。点击「连接直播间」开始接收直播事件。" : "配置已保存并生效。直播接入当前已关闭。";
      setMessage(notice); feedback.success("直播配置已保存", notice);
      await onSaved();
    } catch (reason) {
      if (!controller.signal.aborted) {
        setError(reason instanceof Error ? reason.message : "保存直播配置失败，请重试。");
        feedback.error("保存直播配置失败", reason);
      }
    } finally {
      if (saveController.current === controller) saveController.current = null;
      if (!controller.signal.aborted) { setSaving(false); onSavingChange(false); }
    }
  }

  return <section className="panel live-settings-panel" aria-labelledby="live-settings-heading">
    <h2 id="live-settings-heading">哔哩哔哩直播配置</h2>
    <p className="field-hint">首次使用：准备下面四项信息，启用接入并保存，再点击「连接直播间」。凭据保存在主服务所在电脑，无须重启。</p>
    <p className="field-hint">前往 <a href="https://open-live.bilibili.com/" target="_blank" rel="noreferrer">哔哩哔哩直播开放平台</a> 获取应用信息与接入说明。应用 ID 不是直播间号，主播身份码不是账号密码。</p>
    {loading && <p role="status">正在读取直播配置…</p>}
    {error && <div className="error-banner" role="alert">{error}
      {!snapshot && !loading && <div className="form-actions"><button type="button" onClick={() => setLoadAttempt(value => value + 1)}>重新加载直播配置</button></div>}
    </div>}
    {message && <p className="success-banner" role="status">{message}</p>}
    {snapshot && !snapshot.storage_available && <p className="availability-note">此主服务当前无法保存直播配置，请通过项目启动入口启动主服务后重试。</p>}
    {busyConnection && <p className="availability-note">修改或保存配置前，请先断开直播间。</p>}
    <form onSubmit={event => { void save(event); }} noValidate>
      <label className="checkbox-field"><input type="checkbox" checked={enabled} disabled={unavailable || clearCredentials} onChange={event => { setEnabled(event.target.checked); clearNotice(); }} /><span>启用哔哩哔哩直播接入</span></label>
      <div className="workspace-columns">
        <div>
          <label htmlFor="live-app-id">应用 ID</label>
          <input id="live-app-id" type="text" inputMode="numeric" autoComplete="off" value={appId} disabled={unavailable} onChange={event => { setAppId(event.target.value); clearNotice(); }} placeholder="复制开放平台中的应用 ID" aria-describedby="live-app-id-hint" />
          <p className="field-hint" id="live-app-id-hint">填写应用的数字 ID；更换应用时，需要重新填写下面全部凭据。</p>
          {credentialFields.slice(0, 1).map(field => renderCredential(field))}
        </div>
        <div>{credentialFields.slice(1).map(field => renderCredential(field))}</div>
      </div>
      <label className="checkbox-field"><input type="checkbox" checked={clearCredentials} disabled={unavailable} onChange={event => {
        setClearCredentials(event.target.checked);
        if (event.target.checked) { setEnabled(false); setCredentials(emptyCredentials); }
        clearNotice();
      }} /><span>移除已保存的全部直播凭据</span></label>
      {clearCredentials && <p className="availability-note">保存后会移除 AccessKey ID、AccessKey Secret 和主播身份码，并关闭直播接入；下次使用需要重新填写。</p>}
      <div className="form-actions"><button type="submit" className="primary-button" disabled={unavailable}>{saving ? "正在保存直播配置…" : "保存直播配置"}</button></div>
    </form>
  </section>;

  function renderCredential(field: typeof credentialFields[number]) {
    const stored = Boolean(snapshot?.[`${field.key}_configured`]);
    const hintId = `live-${field.key}-hint`;
    return <div key={field.key}>
      <label htmlFor={`live-${field.key}`}>{field.label}</label>
      <input id={`live-${field.key}`} type="password" autoComplete="new-password" value={credentials[field.key]} disabled={unavailable || clearCredentials} aria-describedby={hintId}
        placeholder={stored && !appChanged ? "留空继续使用已保存凭据" : `粘贴${field.label}`} onChange={event => { setCredentials(current => ({ ...current, [field.key]: event.target.value })); clearNotice(); }} />
      <p className="field-hint" id={hintId}><strong>{stored && !appChanged ? "已保存；留空继续使用" : "尚未填写"}</strong> · {field.help}</p>
    </div>;
  }
}
