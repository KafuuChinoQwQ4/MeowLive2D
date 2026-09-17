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
  const [validationError, setValidationError] = useState<string | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const seconds = Number(cooldownSeconds);
    const messages: string[] = [];
    if (!persona.trim() || [...persona].length > 2_000) messages.push("主播人设须为 1 到 2000 个字符。");
    if ([...topic].length > 200) messages.push("直播话题最多 200 个字符，可以留空。");
    if (!Number.isInteger(seconds) || seconds < 1 || seconds > 3_600) messages.push("冷却时间必须是 1 到 3600 秒的整数。");
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
    });
  }

  return (
    <section className="panel" aria-labelledby="agent-settings-heading">
      <p className="eyebrow">互动策略</p>
      <h2 id="agent-settings-heading">Agent 设置</h2>
      <p className="muted">保存设置会暂停 Agent 并取消正在进行的决策。</p>
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
        {validationError && <p className="field-error" role="alert">{validationError}</p>}
        <div className="form-actions">
          <button className="primary-button" type="submit" disabled={disabled}>保存设置并暂停</button>
        </div>
      </form>
    </section>
  );
}
