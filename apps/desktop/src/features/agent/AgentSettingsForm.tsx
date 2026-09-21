import { useState } from "react";
import type { FormEvent } from "react";
import type { AgentSettings } from "@meowlive/contracts";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export function AgentSettingsForm({ settings, disabled, onSave }: {
  settings: AgentSettings;
  disabled: boolean;
  onSave: (settings: AgentSettings) => Promise<boolean>;
}) {
  const feedback = useFeedback();
  const [persona, setPersona] = useState(settings.persona);
  const [topic, setTopic] = useState(settings.topic);
  const [proactive, setProactive] = useState(settings.proactive_enabled);
  const [cooldownSeconds, setCooldownSeconds] = useState(String(settings.cooldown_ms / 1_000));
  const [chatReadMode, setChatReadMode] = useState(settings.interaction.chat_read_mode);
  const [welcomeEnabled, setWelcomeEnabled] = useState(settings.interaction.welcome_enabled);
  const [busyChatCount, setBusyChatCount] = useState(String(settings.interaction.busy_chat_count));
  const [busyEnterCount, setBusyEnterCount] = useState(String(settings.interaction.busy_enter_count));
  const [busyPendingCount, setBusyPendingCount] = useState(String(settings.interaction.busy_pending_count));
  const [welcomeSeconds, setWelcomeSeconds] = useState(String(settings.interaction.welcome_cooldown_ms / 1_000));
  const [welcomeViewerSeconds, setWelcomeViewerSeconds] = useState(String(settings.interaction.welcome_viewer_cooldown_ms / 1_000));
  const [validationError, setValidationError] = useState<string | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const seconds = Number(cooldownSeconds);
    const messages: string[] = [];
    if (!persona.trim() || [...persona].length > 2_000) messages.push("主播人设须为 1 到 2000 个字符。");
    if ([...topic].length > 200) messages.push("直播话题最多 200 个字符，可以留空。");
    if (!Number.isInteger(seconds) || seconds < 1 || seconds > 3_600) messages.push("冷却时间必须是 1 到 3600 秒的整数。");
    const chatCount = Number(busyChatCount);
    const enterCount = Number(busyEnterCount);
    const pendingCount = Number(busyPendingCount);
    for (const [label, value, max] of [
      ["每分钟弹幕阈值", chatCount, 1000], ["每分钟进房阈值", enterCount, 1000], ["待处理事件阈值", pendingCount, 512],
    ] as const) {
      if (!Number.isInteger(value) || value < 1 || value > max) messages.push(`${label}必须是 1 到 ${max} 的整数。`);
    }
    const welcomeMs = Math.round(Number(welcomeSeconds) * 1_000);
    const welcomeViewerMs = Math.round(Number(welcomeViewerSeconds) * 1_000);
    for (const [label, raw, value, maxSeconds] of [
      ["欢迎间隔", welcomeSeconds, welcomeMs, 3600], ["同一观众欢迎间隔", welcomeViewerSeconds, welcomeViewerMs, 86400],
    ] as const) {
      if (!Number.isSafeInteger(value) || value < 1000 || value > maxSeconds * 1000 || value / 1000 !== Number(raw)) {
        messages.push(`${label}必须为 1 到 ${maxSeconds} 秒，最多三位小数。`);
      }
    }
    if (messages.length > 0) {
      setValidationError(messages.join(" "));
      feedback.error("Agent 设置检查失败", messages.join(" "));
      return;
    }
    setValidationError(null);
    await onSave({
      persona: persona.trim(),
      topic: topic.trim(),
      proactive_enabled: proactive,
      cooldown_ms: seconds * 1_000,
      interaction: {
        chat_read_mode: chatReadMode, welcome_enabled: welcomeEnabled,
        busy_chat_count: chatCount, busy_enter_count: enterCount, busy_pending_count: pendingCount,
        welcome_cooldown_ms: welcomeMs, welcome_viewer_cooldown_ms: welcomeViewerMs,
      },
    });
  }

  return (
    <section className="panel" aria-labelledby="agent-settings-heading">
      <h2 id="agent-settings-heading">Agent 设置</h2>
      <p className="muted">保存设置会暂停 Agent；配置保存在本机，下次打开仍保留。</p>
      <form noValidate onSubmit={(event) => { void submit(event); }}>
        <label htmlFor="agent-persona">主播人设</label>
        <textarea id="agent-persona" rows={4} value={persona} disabled={disabled}
          onChange={(event) => setPersona(event.target.value)} />
        <label htmlFor="agent-topic">直播话题</label>
        <input id="agent-topic" value={topic} disabled={disabled}
          onChange={(event) => setTopic(event.target.value)} />
        <label htmlFor="agent-cooldown">冷却时间（秒）</label>
        <input id="agent-cooldown" type="number" min="1" max="3600" step="1" value={cooldownSeconds} disabled={disabled}
          onChange={(event) => setCooldownSeconds(event.target.value)} />
        <label className="checkbox-field">
          <input type="checkbox" checked={proactive} disabled={disabled}
            onChange={(event) => setProactive(event.target.checked)} />
          <span>允许空闲时主动发言</span>
        </label>
        <div className="panel-divider" />
        <h3>弹幕与欢迎</h3>
        <label htmlFor="agent-chat-read-mode">读弹幕策略</label>
        <select id="agent-chat-read-mode" value={chatReadMode} disabled={disabled}
          onChange={(event) => setChatReadMode(event.target.value as AgentSettings["interaction"]["chat_read_mode"])}>
          <option value="auto">自动：清闲时逐条读出并回复，繁忙时选择回应</option>
          <option value="all">全部：逐条读出并回复</option>
          <option value="selective">选择：由 Agent 挑选弹幕回应</option>
        </select>
        <p className="field-hint">SC 优先读出昵称、金额和留言，再回复；暂停、过期、处理失败或队列已满可能导致未播报。</p>
        <label className="checkbox-field">
          <input type="checkbox" checked={welcomeEnabled} disabled={disabled}
            onChange={(event) => setWelcomeEnabled(event.target.checked)} />
          <span>欢迎进房观众</span>
        </label>
        <p className="field-hint">进房稀疏时按昵称欢迎，繁忙时暂停欢迎。当前依据收到的事件判断活跃程度，并非实时在线人数；大直播间可手动关闭欢迎。</p>
        <div className="compact-fields">
          <div><label htmlFor="agent-busy-chat">每分钟弹幕阈值</label>
            <input id="agent-busy-chat" type="number" min="1" max="1000" step="1" value={busyChatCount} disabled={disabled} onChange={(event) => setBusyChatCount(event.target.value)} /></div>
          <div><label htmlFor="agent-busy-enter">每分钟进房阈值</label>
            <input id="agent-busy-enter" type="number" min="1" max="1000" step="1" value={busyEnterCount} disabled={disabled} onChange={(event) => setBusyEnterCount(event.target.value)} /></div>
          <div><label htmlFor="agent-busy-pending">待处理事件阈值</label>
            <input id="agent-busy-pending" type="number" min="1" max="512" step="1" value={busyPendingCount} disabled={disabled} onChange={(event) => setBusyPendingCount(event.target.value)} /></div>
        </div>
        <p className="field-hint">统计最近 60 秒的弹幕和进房次数；任一计数或当前待处理事件数达到阈值即视为繁忙。</p>
        <div className="compact-fields">
          <div><label htmlFor="agent-welcome-cooldown">欢迎间隔（秒）</label>
            <input id="agent-welcome-cooldown" type="number" min="1" max="3600" step="0.001" value={welcomeSeconds} disabled={disabled} onChange={(event) => setWelcomeSeconds(event.target.value)} /></div>
          <div><label htmlFor="agent-welcome-viewer-cooldown">同一观众欢迎间隔（秒）</label>
            <input id="agent-welcome-viewer-cooldown" type="number" min="1" max="86400" step="0.001" value={welcomeViewerSeconds} disabled={disabled} onChange={(event) => setWelcomeViewerSeconds(event.target.value)} /></div>
        </div>
        {validationError && <p className="field-error" role="alert">{validationError}</p>}
        <div className="form-actions">
          <button className="primary-button" type="submit" disabled={disabled}>保存设置并暂停</button>
        </div>
      </form>
    </section>
  );
}
