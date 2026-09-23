import type { ModelVersion, VoiceProfile } from "@meowlive/contracts";
import { useEffect, useState } from "react";
import type { useTraining } from "./useTraining";

type Controller = ReturnType<typeof useTraining>;
type VoiceGroup = { id: string; name: string; versions: ModelVersion[] };
const PAGE_SIZE = 5;

export function groupTrainingVoices(versions: ModelVersion[], voices: VoiceProfile[]): VoiceGroup[] {
  const groups = new Map<string, VoiceGroup>();
  for (const version of [...versions].reverse().sort((a, b) => b.created_at_ms - a.created_at_ms)) {
    let group = groups.get(version.voice_id);
    if (!group) {
      group = { id: version.voice_id, name: voices.find(voice => voice.id === version.voice_id)?.name ?? `参考音色缺失（${version.voice_id}）`, versions: [] };
      groups.set(group.id, group);
    }
    group.versions.push(version);
  }
  return [...groups.values()];
}

export function TrainingVoiceLibrary({ controller: c, disabled, canSynthesize, text, onText, deleteVersion }: {
  controller: Controller; disabled: boolean; canSynthesize: boolean; text: string; onText: (text: string) => void;
  deleteVersion: (id: string, name: string) => void;
}) {
  const groups = groupTrainingVoices(c.snapshot?.versions ?? [], c.voices);
  const savedGroups = groups.map(group => ({ ...group, versions: group.versions.filter(version => version.saved) })).filter(group => group.versions.length);
  const [selectedVoice, setSelectedVoice] = useState("");
  const [selectedId, setSelectedId] = useState("");
  const [page, setPage] = useState(0);
  const selectedGroup = savedGroups.find(group => group.id === selectedVoice);
  const selected = selectedGroup?.versions.find(version => version.id === selectedId)
    ?? selectedGroup?.versions.find(version => version.available) ?? selectedGroup?.versions[0];
  const current = c.snapshot?.versions.find(version => version.active && version.voice_id === c.activeVoiceId);
  const hasReference = (id: string) => c.voices.some(voice => voice.id === id && voice.available);
  const pages = Math.max(1, Math.ceil(groups.length / PAGE_SIZE));
  const currentPage = Math.min(page, pages - 1);
  useEffect(() => {
    if (selectedVoice && !c.snapshot?.versions.some(version => version.voice_id === selectedVoice && version.saved)) {
      setSelectedVoice(""); setSelectedId("");
    }
  }, [c.snapshot, selectedVoice]);
  return <>
    <section className="panel" aria-labelledby="saved-voices-heading">
      <h3 id="saved-voices-heading">已保存的训练音色</h3>
      <p className="muted">先选择音色和版本，再点击确认；下拉选择本身不会切换播报音色。</p>
      <p role="status" className="voice-selection-current">当前使用：{current ? `${c.voices.find(voice => voice.id === current.voice_id)?.name ?? "参考音色缺失"} · ${current.name}` : c.activeVoiceId === undefined || !c.snapshot ? "正在读取…" : "尚未选用训练音色"}</p>
      {savedGroups.length === 0 && <p className="field-hint">完成训练并试听后，即可保存音色。</p>}
      <label>选择已保存音色<select value={selectedGroup?.id ?? ""} disabled={disabled || savedGroups.length === 0}
        onChange={event => { setSelectedVoice(event.target.value); setSelectedId(""); }}>
        <option value="">选择一个已保存音色</option>
        {savedGroups.map(group => <option key={group.id} value={group.id} disabled={!hasReference(group.id) || !group.versions.some(version => version.available)}>{group.name}{!hasReference(group.id) ? "（参考音频不可用）" : !group.versions.some(version => version.available) ? "（模型不可用）" : ""}</option>)}
      </select></label>
      {selectedGroup && <label>选择已保存版本<select value={selected?.id ?? ""} disabled={disabled} onChange={event => setSelectedId(event.target.value)}>
        {selectedGroup.versions.map(version => <option key={version.id} value={version.id} disabled={!version.available}>{version.name}{version.id === current?.id ? "（当前使用）" : ""}{!version.available ? "（模型不可用）" : ""}</option>)}
      </select></label>}
      {selected && selected.id !== current?.id && <p className="voice-selection-pending">待确认：{selectedGroup?.name} · {selected.name}</p>}
      <div className="form-actions"><button type="button" className="primary-button"
        disabled={disabled || !selected?.available || !hasReference(selected.voice_id) || current?.id === selected.id}
        onClick={() => { if (selected) void c.activateVersion(selected); }}>确认所选音色</button>
        <button type="button" disabled={disabled || !selected} onClick={() => { if (selected) deleteVersion(selected.id, selected.name); }}>删除所选版本</button></div>
      <p className="field-hint">删除仅影响所选版本；播报前需启用语音模型。</p>
    </section>
    <label>试听与测量文本<textarea value={text} maxLength={500} onChange={event => onText(event.target.value)} /></label>
    {!canSynthesize && <p className="availability-note">试听前请先启动 TTS 服务并启用上方语音模型。</p>}
    <ul className="training-list" aria-label="已训练音色">{groups.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE).map(group => <TrainingVoiceCard key={group.id} group={group} controller={c} disabled={disabled} canSynthesize={canSynthesize}
      referenceAvailable={hasReference(group.id)} text={text} deleteVersion={deleteVersion} />)}</ul>
    {groups.length === 0 && <p className="field-hint">还没有训练完成的音色。</p>}
    <nav className="training-pagination" aria-label="已训练音色分页"><span>共 {groups.length} 个音色 · 第 {currentPage + 1} / {pages} 页</span>
      <button type="button" aria-label="已训练音色上一页" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>上一页</button>
      <button type="button" aria-label="已训练音色下一页" disabled={currentPage + 1 >= pages} onClick={() => setPage(currentPage + 1)}>下一页</button></nav>
  </>;
}

function TrainingVoiceCard({ group, controller: c, disabled, canSynthesize, referenceAvailable, text, deleteVersion }: {
  group: VoiceGroup; controller: Controller; disabled: boolean; canSynthesize: boolean; referenceAvailable: boolean; text: string;
  deleteVersion: (id: string, name: string) => void;
}) {
  const [selectedId, setSelectedId] = useState("");
  const [listened, setListened] = useState<string[]>([]);
  const version = group.versions.find(version => version.id === selectedId) ?? group.versions[0];
  const current = group.versions.find(version => version.active);
  const selected = version.active && c.activeVoiceId === group.id;
  return <li><strong>{group.name}</strong> · {group.versions.length} 次训练
    {current && <span> · {c.activeVoiceId === group.id ? "当前选用" : "已选版本"}：{current.name}</span>}
    <p>查看版本：{version.name} · {new Date(version.created_at_ms).toLocaleString()}</p>
    <details className="training-details"><summary>查看训练版本（{group.versions.length}）</summary>
      <label>{group.name} 的训练版本<select value={version.id} disabled={disabled} onChange={event => setSelectedId(event.target.value)}>
        {group.versions.map((item, index) => <option key={item.id} value={item.id}>{item.name}{index === 0 ? "（最新）" : ""}{item.saved ? " · 已保存" : " · 待确认"}</option>)}
      </select></label>
      <p className="field-hint">各版本独立保留，可试听、切换或删除。</p>
    </details>
    {!version.available && <p className="error-banner">模型文件缺失或损坏</p>}
    {!referenceAvailable && <p className="field-error">参考音频不可用，请检查音色资源。</p>}
    <div className="resource-actions"><button type="button" disabled={disabled || !canSynthesize || !version.available || !referenceAvailable || !text.trim()} onClick={() => { void c.audition(version.id, text); }}>生成版本试听</button>
      <button type="button" disabled={disabled || version.saved || !version.available || !version.auditioned || !listened.includes(version.id)} onClick={() => { void c.saveVersion(version.id); }}>{version.saved ? "音色已保存" : "保存音色"}</button>
      <button type="button" disabled={disabled || !version.available || !referenceAvailable || !version.auditioned || (!version.saved && !listened.includes(version.id)) || selected} onClick={() => { void c.activateVersion(version); }}>选用此版本</button>
      <button type="button" disabled={disabled} aria-label={`删除版本 ${version.name}`} onClick={() => deleteVersion(version.id, version.name)}>删除版本</button></div>
    {c.audio?.version === version.id && <><audio controls src={c.audio.url} aria-label="版本试听音频" /><label><input type="checkbox" checked={listened.includes(version.id)} onChange={event => setListened(previous => event.target.checked ? [...previous, version.id] : previous.filter(id => id !== version.id))} />我已试听并确认此版本效果</label></>}
  </li>;
}
