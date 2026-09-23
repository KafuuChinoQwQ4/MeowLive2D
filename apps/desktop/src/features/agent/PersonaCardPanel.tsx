import { useEffect, useRef, useState, type FormEvent } from "react";
import type { AgentSettings } from "@meowlive/contracts";
import { createAgentClient, type AgentClient } from "../../services/server/agent";
import { useFeedback } from "../../app/feedback/OperationFeedback";
import { parsePersonaCard, serializePersonaCard, type PersonaCard } from "./personaCard";

const defaultClient = createAgentClient();
const details: { key: keyof PersonaCard; label: string; hint: string; lines?: number }[] = [
  { key: "name", label: "角色姓名与称呼", hint: "名字、昵称、希望观众如何称呼你" },
  { key: "personality", label: "性格特点", hint: "例如：开朗、细心、偶尔毒舌但不伤人" },
  { key: "background", label: "背景经历", hint: "角色来自哪里，有什么经历和当前目标", lines: 2 },
  { key: "speechStyle", label: "说话风格", hint: "语气、句长、口头禅、是否使用表情或网络用语", lines: 2 },
  { key: "interests", label: "兴趣与擅长", hint: "喜欢聊什么，熟悉哪些游戏、作品或技能", lines: 2 },
  { key: "viewerRelationship", label: "与观众的关系", hint: "把观众视作朋友、同伴、学生或冒险队员等", lines: 2 },
  { key: "interactionHabits", label: "互动习惯", hint: "如何接梗、提问、感谢礼物、安慰或鼓励观众", lines: 2 },
  { key: "values", label: "价值观与目标", hint: "角色重视什么，希望直播间保持怎样的氛围", lines: 2 },
  { key: "boundaries", label: "互动禁忌与边界", hint: "不想谈的话题、不能破坏的设定和应避免的表达", lines: 2 },
  { key: "examples", label: "示例表达", hint: "写 2～3 句符合角色口吻的示例，每句单独一行", lines: 3 },
];

export function PersonaCardPanel({ client = defaultClient }: { client?: AgentClient }) {
  const feedback = useFeedback();
  const [settings, setSettings] = useState<AgentSettings | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const active = useRef<AbortController | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    active.current = controller;
    void client.getStatus(controller.signal).then(snapshot => {
      if (!controller.signal.aborted) { setSettings(snapshot.settings); setError(""); }
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : "人物卡读取失败");
    });
    return () => { controller.abort(); active.current?.abort(); active.current = null; };
  }, [client, attempt]);

  async function save(persona: string) {
    if (busy) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true); setError("");
    try {
      const current = await client.getStatus(controller.signal);
      const saved = await client.saveSettings({ ...current.settings, persona }, controller.signal);
      if (!controller.signal.aborted) {
        if (saved.last_error) throw new Error(saved.last_error);
        if (!saved.paused || saved.settings.persona !== persona) throw new Error("主服务未确认人物卡保存，请检查 Agent 状态后重试。");
        setSettings(saved.settings);
        feedback.success("人物卡已保存", "Agent 已暂停；确认设置后可到 Agent 互动恢复。");
      }
    } catch (reason) {
      if (!controller.signal.aborted) {
        const message = reason instanceof Error ? reason.message : "人物卡保存失败";
        setError(message); feedback.error("人物卡保存失败", message);
      }
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      if (active.current === controller) active.current = null;
    }
  }

  return <section className="panel persona-card-panel" aria-labelledby="persona-card-heading">
    <div className="section-title"><div><h2 id="persona-card-heading">主播人物卡</h2><p className="field-hint">与左侧角色形象一起设置；这张人物卡是 Agent 的全局人设，切换角色不会自动切换人物卡。</p></div></div>
    {error && <p className="error-banner" role="alert">{error}</p>}
    {!settings && (error ? <button type="button" onClick={() => setAttempt(value => value + 1)}>重新读取人物卡</button> : <p role="status">正在读取人物卡…</p>)}
    {settings && <PersonaCardEditor key={`${client.baseUrl}:${settings.persona}`} persona={settings.persona} disabled={busy} onSave={save} />}
  </section>;
}

function PersonaCardEditor({ persona, disabled, onSave }: { persona: string; disabled: boolean; onSave: (persona: string) => Promise<void> }) {
  const [card, setCard] = useState(() => parsePersonaCard(persona));
  const [expanded, setExpanded] = useState(false);
  const [validationError, setValidationError] = useState("");
  const serialized = serializePersonaCard(card);
  const length = [...serialized].length;
  const change = (key: keyof PersonaCard, value: string) => setCard(current => ({ ...current, [key]: value }));
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!card.identity.trim() || length > 2_000) {
      setValidationError("主播人设须填写核心身份，且完整人物卡不超过 2000 个字符。");
      return;
    }
    setValidationError("");
    void onSave(serialized.trim());
  }
  return <form noValidate onSubmit={submit}>
    <div className="persona-card-heading"><p className="field-hint">先写核心身份，再按需补充表达方式和互动边界。</p>
      <span className={length > 2_000 ? "persona-card-count is-over" : "persona-card-count"}>{length} / 2000</span></div>
    <label htmlFor="persona-identity">核心身份</label>
    <textarea id="persona-identity" rows={4} value={card.identity} disabled={disabled}
      aria-invalid={!card.identity.trim() || length > 2_000}
      placeholder="例如：你是一位在 bilibili 直播的魔法少女，声音温柔治愈。"
      onChange={event => change("identity", event.target.value)} />
    <details className="persona-card-details" open={expanded} onToggle={event => setExpanded(event.currentTarget.open)}>
      <summary>{expanded ? "收起人物卡" : "展开人物卡"}</summary>
      <p className="field-hint">以下均为可选项。用具体描述和示例，比堆叠形容词更容易得到稳定表现。</p>
      <div className="persona-card-grid">{details.map(field => <label htmlFor={`persona-${field.key}`} key={field.key}>{field.label}
        {field.lines
          ? <textarea id={`persona-${field.key}`} rows={field.lines} value={card[field.key]} disabled={disabled} placeholder={field.hint} onChange={event => change(field.key, event.target.value)} />
          : <input id={`persona-${field.key}`} value={card[field.key]} disabled={disabled} placeholder={field.hint} onChange={event => change(field.key, event.target.value)} />}
      </label>)}</div>
    </details>
    {validationError && <p className="field-error" role="alert">{validationError}</p>}
    <p className="field-hint">保存人物卡会暂停 Agent，其他互动设置保持原值。</p>
    <div className="form-actions"><button className="primary-button" type="submit" disabled={disabled}>保存人物卡并暂停 Agent</button></div>
  </form>;
}
