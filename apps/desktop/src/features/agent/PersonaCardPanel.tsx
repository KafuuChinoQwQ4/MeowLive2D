import { useEffect, useRef, useState, type FormEvent } from "react";
import type { AgentSettings, PersonaProfilesSnapshot } from "@meowlive/contracts";
import { createAgentClient, type AgentClient } from "../../services/server/agent";
import { useFeedback } from "../../app/feedback/OperationFeedback";
import { emptyPersonaCard, parsePersonaCard, serializePersonaCard, type PersonaCard } from "./personaCard";

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

function nextProfileName(profiles: PersonaProfilesSnapshot["profiles"]): string {
  for (let number = 1; number <= profiles.length + 1; number += 1) {
    const name = `配置${number}`;
    if (!profiles.some(profile => profile.name === name)) return name;
  }
  return `配置${profiles.length + 1}`;
}

export function PersonaCardPanel({ client = defaultClient }: { client?: AgentClient }) {
  const feedback = useFeedback();
  const [settings, setSettings] = useState<AgentSettings | null>(null);
  const [profiles, setProfiles] = useState<PersonaProfilesSnapshot | null>(null);
  const [profileId, setProfileId] = useState<string | null>(null);
  const [profileName, setProfileName] = useState("");
  const [originalName, setOriginalName] = useState("");
  const [card, setCard] = useState<PersonaCard>(emptyPersonaCard);
  const [originalPersona, setOriginalPersona] = useState("");
  const [profilesExpanded, setProfilesExpanded] = useState(false);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const active = useRef<AbortController | null>(null);

  function applySnapshot(snapshot: PersonaProfilesSnapshot, current: AgentSettings) {
    const selected = snapshot.profiles.find(profile => profile.id === snapshot.selected_profile_id);
    const persona = selected?.persona ?? current.persona;
    setSettings(current);
    setProfiles(snapshot);
    setProfileId(snapshot.selected_profile_id);
    const name = selected?.name ?? nextProfileName(snapshot.profiles);
    setProfileName(name);
    setOriginalName(name);
    setCard(parsePersonaCard(persona));
    setOriginalPersona(persona);
    setProfilesExpanded(false);
  }

  useEffect(() => {
    const controller = new AbortController();
    active.current = controller;
    void Promise.all([client.getStatus(controller.signal), client.getPersonaProfiles(controller.signal)]).then(([status, snapshot]) => {
      if (!controller.signal.aborted) { applySnapshot(snapshot, status.settings); setError(""); }
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : "人物卡读取失败");
    });
    return () => { controller.abort(); active.current?.abort(); active.current = null; };
  }, [client, attempt]);

  const dirty = originalPersona !== serializePersonaCard(card).trim() || profileName !== originalName;
  function discardDraft(): boolean {
    return !dirty || window.confirm("当前人物卡有未保存的修改，确定放弃吗？");
  }

  async function run(action: (signal: AbortSignal) => Promise<void>, failure: string) {
    if (busy || !profiles?.storage_available) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true); setError("");
    try {
      await action(controller.signal);
    } catch (reason) {
      if (!controller.signal.aborted) {
        const message = reason instanceof Error ? reason.message : failure;
        setError(message); feedback.error(failure, message);
      }
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      if (active.current === controller) active.current = null;
    }
  }

  async function save(persona: string) {
    const name = profileName.trim();
    if (!name || [...name].length > 64) { setError("人物卡标题须为 1 到 64 个字符。"); return; }
    await run(async signal => {
      if (profileId) {
        const saved = await client.updatePersonaProfile({ id: profileId, name, persona }, signal);
        const current = await client.getStatus(signal);
        if (!current.paused || current.settings.persona !== persona) throw new Error("主服务未确认人物卡保存，请检查 Agent 状态后重试。");
        let snapshot = saved;
        if (!signal.aborted) {
          applySnapshot(snapshot, current.settings);
        }
      } else {
        const snapshot = await client.createPersonaProfile({ name, persona }, signal);
        const current = await client.getStatus(signal);
        if (!current.paused || current.settings.persona !== persona || !snapshot.selected_profile_id) throw new Error("主服务未确认人物卡保存，请检查 Agent 状态后重试。");
        if (!signal.aborted) applySnapshot(snapshot, current.settings);
      }
      if (!signal.aborted) feedback.success("人物卡已保存", "已保存到本机并暂停 Agent。");
    }, "人物卡保存失败");
  }

  function startNew() {
    if (busy || !discardDraft()) return;
    const name = nextProfileName(profiles?.profiles ?? []);
    setProfileId(null);
    setProfileName(name);
    setOriginalName(name);
    setCard(emptyPersonaCard());
    setOriginalPersona("");
    setProfilesExpanded(false);
    setError("");
  }

  async function selectProfile(id: string) {
    if (busy || id === profileId || !discardDraft()) return;
    await run(async signal => {
      const snapshot = await client.selectPersonaProfile({ id }, signal);
      const current = await client.getStatus(signal);
      if (!current.paused || current.settings.persona !== snapshot.profiles.find(profile => profile.id === id)?.persona) throw new Error("主服务未确认人物卡切换。");
      if (!signal.aborted) { applySnapshot(snapshot, current.settings); feedback.success("人物卡已切换", "Agent 已暂停，可在互动页恢复。"); }
    }, "切换人物卡失败");
  }

  async function renameProfile() {
    if (!profileId || busy || !profiles) return;
    const name = profileName.trim();
    if (!name || [...name].length > 64) { setError("人物卡标题须为 1 到 64 个字符。"); return; }
    if (profiles.profiles.find(profile => profile.id === profileId)?.name === name) return;
    await run(async signal => {
      const snapshot = await client.renamePersonaProfile({ id: profileId, name }, signal);
      if (!signal.aborted) { setProfiles(snapshot); setProfileName(name); setOriginalName(name); feedback.success("人物卡已重命名", name); }
    }, "重命名人物卡失败");
  }

  async function deleteProfile() {
    if (!profileId || busy || !window.confirm("删除当前人物卡？此操作不可撤销。")) return;
    await run(async signal => {
      const snapshot = await client.deletePersonaProfile({ id: profileId }, signal);
      const current = await client.getStatus(signal);
      if (!signal.aborted) { applySnapshot(snapshot, current.settings); feedback.success("人物卡已删除", "已更新当前人物卡。"); }
    }, "删除人物卡失败");
  }

  return <section className="panel persona-card-panel" aria-labelledby="persona-card-heading">
    <div className="section-title"><div><h2 id="persona-card-heading">主播人物卡</h2><p className="field-hint">人物卡保存在本机。当前选中卡用于 Agent；切换左侧角色形象不会自动切换人物卡。</p></div></div>
    {error && <p className="error-banner" role="alert">{error}</p>}
    {!settings && (error ? <button type="button" onClick={() => setAttempt(value => value + 1)}>重新读取人物卡</button> : <p role="status">正在读取人物卡…</p>)}
    {settings && profiles && <>
      {!profiles.storage_available && <p className="availability-note">此主服务未启用人物卡保存，请使用项目主服务启动入口。</p>}
      <div className="profile-toolbar">
        <label htmlFor="persona-profile-name">人物卡标题</label>
        <div className="profile-picker">
          <div className="profile-picker-control">
            <input id="persona-profile-name" value={profileName} maxLength={64} disabled={busy || !profiles.storage_available} onChange={event => setProfileName(event.target.value)} />
            <button type="button" className="profile-toggle" aria-expanded={profilesExpanded} aria-controls="persona-profile-menu" disabled={busy} onClick={() => setProfilesExpanded(value => !value)}>已保存人物卡 <span className="profile-chevron" aria-hidden="true">⌄</span></button>
          </div>
          {profilesExpanded && <div className="profile-list" id="persona-profile-menu" aria-label="已保存人物卡列表">
            {profiles.profiles.length ? profiles.profiles.map(profile => <button type="button" className="profile-option" key={profile.id} aria-pressed={profile.id === profiles.selected_profile_id} disabled={busy || !profiles.storage_available} onClick={() => { void selectProfile(profile.id); }}>
              <span className="profile-option-name">{profile.name}</span><span className="profile-option-detail">{parsePersonaCard(profile.persona).identity}</span>
            </button>) : <p className="field-hint">暂无已保存人物卡</p>}
            <div className="profile-menu-actions form-actions">
              {profileId && <button type="button" disabled={busy || !profiles.storage_available} onClick={() => { void deleteProfile(); }}>删除人物卡</button>}
              <button type="button" disabled={busy || !profiles.storage_available} onClick={startNew}>新建人物卡</button>
            </div>
          </div>}
          {profileId && <button type="button" className="persona-rename-button" disabled={busy || !profiles.storage_available || profileName.trim() === profiles.profiles.find(profile => profile.id === profileId)?.name} onClick={() => { void renameProfile(); }}>重命名人物卡</button>}
        </div>
      </div>
      <PersonaCardEditor card={card} onChange={setCard} disabled={busy || !profiles.storage_available} onSave={save} />
    </>}
  </section>;
}

function PersonaCardEditor({ card, onChange, disabled, onSave }: { card: PersonaCard; onChange: (card: PersonaCard) => void; disabled: boolean; onSave: (persona: string) => Promise<void> }) {
  const [expanded, setExpanded] = useState(false);
  const [validationError, setValidationError] = useState("");
  const serialized = serializePersonaCard(card);
  const length = [...serialized].length;
  const change = (key: keyof PersonaCard, value: string) => { onChange({ ...card, [key]: value }); setValidationError(""); };
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
