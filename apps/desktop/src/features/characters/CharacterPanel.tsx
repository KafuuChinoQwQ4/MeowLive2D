import { useMemo, useState } from "react";
import type { FormEvent } from "react";
import type { CharacterMapping, CharacterProfile, CharacterSaveRequest } from "@meowlive/contracts";
import type { CharacterController } from "./types";

const emptyCharacter = (): CharacterSaveRequest => ({
  id: null,
  name: "",
  model_id: "",
  voice_id: "",
  mouth_parameter: "MeowMouthOpen",
  mappings: [],
});

function editable(character: CharacterProfile): CharacterSaveRequest {
  return { ...character, mappings: character.mappings.map((mapping) => ({ ...mapping })) };
}

export function CharacterPanel({ controller }: { controller: CharacterController }) {
  const [form, setForm] = useState<CharacterSaveRequest>(emptyCharacter);
  const [dirty, setDirty] = useState(false);
  const snapshot = controller.snapshot;
  const busy = controller.pendingAction !== null;
  const knownModel = controller.models.some((model) => model.id === form.model_id);
  const hotkeys = controller.hotkeyModelId === form.model_id ? controller.hotkeys : [];
  const voiceOptions = useMemo(() => [
    ...(snapshot?.default_voice_available ? [{ id: "default", name: "配置的默认音色" }] : []),
    ...(snapshot?.voices.filter((voice) => voice.available).map((voice) => ({ id: voice.id, name: voice.name })) ?? []),
  ], [snapshot]);
  const valid = form.name.trim().length > 0 && form.name.trim().length <= 80
    && form.model_id.trim().length > 0 && /^[A-Za-z0-9]{4,32}$/u.test(form.mouth_parameter)
    && voiceOptions.some((voice) => voice.id === form.voice_id)
    && form.mappings.length <= 32
    && form.mappings.every((mapping) => mapping.intent.trim() && mapping.hotkey_id.trim())
    && new Set(form.mappings.map((mapping) => mapping.intent.trim())).size === form.mappings.length;

  function change(patch: Partial<CharacterSaveRequest>) {
    setForm((current) => ({ ...current, ...patch }));
    setDirty(true);
  }

  function changeMapping(index: number, patch: Partial<CharacterMapping>) {
    change({ mappings: form.mappings.map((mapping, item) => item === index ? { ...mapping, ...patch, validated: false } : mapping) });
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!valid || busy) return;
    const previousIds = new Set(snapshot?.characters.map((character) => character.id));
    const value = await controller.saveCharacter({
      ...form,
      name: form.name.trim(),
      model_id: form.model_id.trim(),
      voice_id: form.voice_id,
      mouth_parameter: form.mouth_parameter.trim(),
      mappings: form.mappings.map((mapping) => ({
        intent: mapping.intent.trim(),
        hotkey_id: mapping.hotkey_id.trim(),
        fallback_hotkey_id: mapping.fallback_hotkey_id?.trim() || null,
        validated: mapping.validated,
      })),
    });
    if (!value) return;
    const saved = form.id
      ? value.characters.find((character) => character.id === form.id)
      : value.characters.find((character) => !previousIds.has(character.id));
    if (saved) setForm(editable(saved));
    setDirty(false);
  }

  async function preview(mapping: CharacterMapping) {
    if (!form.id || dirty) return;
    const value = await controller.previewCharacter({ character_id: form.id, intent: mapping.intent });
    const updated = value?.characters.find((character) => character.id === form.id);
    if (updated) setForm(editable(updated));
  }

  return <section className="panel" aria-labelledby="characters-heading">
    <div className="section-title">
      <div><p className="eyebrow">形象资源</p><h2 id="characters-heading">角色管理</h2></div>
      <button type="button" disabled={busy} onClick={() => { void controller.refreshModels(); }}>
        {controller.pendingAction === "models" ? "正在刷新…" : "刷新 VTS 模型"}
      </button>
    </div>
    <p className="muted">保存角色配置后，选择角色会在 VTube Studio 中加载对应模型。</p>

    <ul className="resource-list" aria-label="已保存角色">
      {snapshot?.characters.map((character) => <li className="resource-row" key={character.id}>
        <div className="resource-row-heading"><div><strong>{character.name}</strong><span className="field-hint">{character.model_id}</span></div>
          {snapshot.active_character_id === character.id && <span className="task-status task-completed">当前角色</span>}</div>
        <div className="compact-actions">
          <button type="button" disabled={busy} onClick={() => { setForm(editable(character)); setDirty(false); }}>编辑</button>
          <button type="button" className="primary-button" disabled={busy}
            onClick={() => { void controller.selectCharacter(character.id); }}>
            {snapshot.active_character_id === character.id ? "重新加载" : "选择并加载"}
          </button>
        </div>
      </li>)}
    </ul>

    <div className="model-actions">
      <button type="button" disabled={busy} onClick={() => { setForm(emptyCharacter()); setDirty(false); }}>新建角色</button>
      <button type="button" disabled={busy} onClick={() => { void controller.importModel(); }}>
        {controller.pendingAction === "model-import" ? "正在导入…" : "从本机导入模型"}
      </button>
    </div>
    {controller.importResult && <p className="availability-note" role="status">
      已导入 {controller.importResult.model_name}（{controller.importResult.files} 个文件），请重启 VTube Studio 后刷新模型列表。
    </p>}

    <div className="panel-divider" />
    <h3>{form.id ? "编辑角色" : "新建角色"}</h3>
    <form onSubmit={(event) => { void save(event); }}>
      <label htmlFor="character-name">角色名称</label>
      <input id="character-name" maxLength={80} value={form.name} disabled={busy}
        onChange={(event) => change({ name: event.target.value })} />

      <label htmlFor="character-model">VTS 模型</label>
      <div className="inline-field-action">
        <select id="character-model" value={form.model_id} disabled={busy}
          onChange={(event) => change({ model_id: event.target.value, mappings: [] })}>
          <option value="">请选择模型</option>
          {form.model_id && !knownModel && <option value={form.model_id}>{form.model_id}</option>}
          {controller.models.map((model) => <option key={model.id} value={model.id}>{model.name}</option>)}
        </select>
        <button type="button" disabled={busy || !form.model_id}
          onClick={() => { void controller.refreshHotkeys(form.model_id); }}>刷新热键</button>
      </div>

      <label htmlFor="character-voice">角色音色</label>
      <select id="character-voice" value={form.voice_id} disabled={busy}
        onChange={(event) => change({ voice_id: event.target.value })}>
        <option value="">请选择音色</option>
        {form.voice_id && !voiceOptions.some((voice) => voice.id === form.voice_id) && <option value={form.voice_id}>当前音色不可用</option>}
        {voiceOptions.map((voice) => <option key={voice.id} value={voice.id}>{voice.name}</option>)}
      </select>
      {snapshot && voiceOptions.length === 0 && <p className="field-hint">请先在音色管理中上传参考声音，再为角色选择音色。</p>}

      <label htmlFor="mouth-parameter">嘴型参数</label>
      <input id="mouth-parameter" value={form.mouth_parameter} disabled={busy}
        aria-invalid={!/^[A-Za-z0-9]{4,32}$/u.test(form.mouth_parameter)}
        onChange={(event) => change({ mouth_parameter: event.target.value })} />
      <p className="field-hint">使用 4–32 位字母或数字，与模型的嘴型参数保持一致。</p>

      <div className="mapping-heading"><h4>意图与热键</h4><button type="button" disabled={busy || form.mappings.length >= 32}
        onClick={() => change({ mappings: [...form.mappings, { intent: "", hotkey_id: "", fallback_hotkey_id: null, validated: false }] })}>添加映射</button></div>
      {form.mappings.map((mapping, index) => <fieldset className="mapping-row" key={index}>
        <legend>映射 {index + 1}</legend>
        <label htmlFor={`intent-${index}`}>意图</label>
        <input id={`intent-${index}`} maxLength={40} value={mapping.intent} disabled={busy}
          onChange={(event) => changeMapping(index, { intent: event.target.value })} />
        <label htmlFor={`hotkey-${index}`}>主要热键</label>
        <select id={`hotkey-${index}`} value={mapping.hotkey_id} disabled={busy}
          onChange={(event) => changeMapping(index, { hotkey_id: event.target.value })}>
          <option value="">请选择热键</option>
          {mapping.hotkey_id && !hotkeys.some((hotkey) => hotkey.id === mapping.hotkey_id) && <option value={mapping.hotkey_id}>{mapping.hotkey_id}</option>}
          {hotkeys.map((hotkey) => <option key={hotkey.id} value={hotkey.id}>{hotkey.name}</option>)}
        </select>
        <label htmlFor={`fallback-${index}`}>备用热键</label>
        <select id={`fallback-${index}`} value={mapping.fallback_hotkey_id ?? ""} disabled={busy}
          onChange={(event) => changeMapping(index, { fallback_hotkey_id: event.target.value || null })}>
          <option value="">无</option>
          {mapping.fallback_hotkey_id && !hotkeys.some((hotkey) => hotkey.id === mapping.fallback_hotkey_id) && <option value={mapping.fallback_hotkey_id}>{mapping.fallback_hotkey_id}</option>}
          {hotkeys.map((hotkey) => <option key={hotkey.id} value={hotkey.id}>{hotkey.name}</option>)}
        </select>
        <div className="mapping-actions">
          <span className={`task-status ${mapping.validated ? "task-completed" : ""}`}>{mapping.validated ? "已验证" : "未验证"}</span>
          <button type="button" disabled={busy || !form.id || dirty || !mapping.intent.trim()}
            onClick={() => { void preview(mapping); }}>预览验证</button>
          <button type="button" className="stop-button" aria-label={`删除映射 ${index + 1}`} title="删除映射" disabled={busy}
            onClick={() => change({ mappings: form.mappings.filter((_, item) => item !== index) })}>×</button>
        </div>
      </fieldset>)}
      {form.id && dirty && <p className="field-hint">请先保存修改，再预览验证热键。</p>}
      <div className="form-actions"><button className="primary-button" type="submit" disabled={busy || !valid}>
        {controller.pendingAction === "character-save" ? "正在保存…" : "保存角色"}
      </button></div>
    </form>
  </section>;
}
