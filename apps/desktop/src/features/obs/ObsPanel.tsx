import { useEffect, useRef, useState } from "react";
import type { ObsOperation, ObsSettingsSnapshot, ObsSnapshot } from "@meowlive/contracts";
import { type ObsClient } from "../../services/server/obs";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export function ObsPanel({ client }: { client: ObsClient }) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<ObsSnapshot | null>(null);
  const [selected, setSelected] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [settings, setSettings] = useState<ObsSettingsSnapshot | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [address, setAddress] = useState("ws://127.0.0.1:4455");
  const [password, setPassword] = useState("");
  const [clearPassword, setClearPassword] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [savedNotice, setSavedNotice] = useState(false);
  const current = useRef<AbortController | null>(null);
  const dirty = settings !== null && (enabled !== settings.enabled || address !== settings.websocket_url || password !== "" || clearPassword);
  function acceptSettings(value: ObsSettingsSnapshot) {
    setSettings(value);
    setEnabled(value.enabled);
    setAddress(value.websocket_url);
    setPassword("");
    setClearPassword(false);
    setSettingsError(null);
  }
  function accept(value: ObsSnapshot) {
    setSnapshot(value);
    setSelected(value.current_scene);
    setError(null);
  }
  useEffect(() => {
    const controller = new AbortController();
    current.current = controller;
    setBusy(true);
    setSnapshot(null);
    setSettings(null);
    setSettingsError(null);
    setPassword("");
    setSavedNotice(false);
    void client.getSettings(controller.signal).then((value) => {
      if (!controller.signal.aborted) { acceptSettings(value); feedback.clearIssue("obs:settings"); }
    }).catch((failure: unknown) => {
      if (!controller.signal.aborted) { setSettingsError(failure instanceof Error ? failure.message : "OBS 设置读取失败"); feedback.reportIssue("obs:settings", "OBS 设置读取失败", failure); }
    }).then(() => {
      controller.signal.throwIfAborted();
      return client.getStatus(controller.signal);
    }).then((value) => {
      if (!controller.signal.aborted) { accept(value); feedback.clearIssue("obs:status"); }
    }).catch((failure: unknown) => {
      if (!controller.signal.aborted) { setError(failure instanceof Error ? failure.message : "OBS 状态读取失败"); feedback.reportIssue("obs:status", "OBS 状态读取失败", failure); }
    }).finally(() => {
      if (!controller.signal.aborted) { current.current = null; setBusy(false); }
    });
    return () => { controller.abort(); current.current?.abort(); };
  }, [client, feedback]);
  async function reloadSettings() {
    if (current.current || dirty) return;
    const controller = new AbortController();
    current.current = controller;
    setBusy(true);
    setSettingsError(null);
    try {
      const value = await client.getSettings(controller.signal);
      if (!controller.signal.aborted) {
        acceptSettings(value);
        setSnapshot(null);
        setSelected("");
        setSavedNotice(false);
        feedback.clearIssue("obs:settings");
      }
    } catch (failure) {
      if (!controller.signal.aborted) setSettingsError(failure instanceof Error ? failure.message : "OBS 设置读取失败");
    } finally {
      if (!controller.signal.aborted) { current.current = null; setBusy(false); }
    }
  }
  async function saveSettings() {
    if (current.current || !settings?.storage_available) return;
    if (address !== settings.websocket_url && password === "" && !clearPassword) {
      setSettingsError("更改 OBS 地址后，请重新输入密码或勾选清除已保存密码。");
      return;
    }
    const controller = new AbortController();
    current.current = controller;
    setBusy(true);
    setSettingsError(null);
    setSavedNotice(false);
    try {
      const value = await client.saveSettings({ enabled, websocket_url: address, password: password || null, clear_password: clearPassword }, controller.signal);
      if (!controller.signal.aborted) {
        acceptSettings(value);
        setSnapshot(null);
        setSelected("");
        setError(null);
        setSavedNotice(true);
        feedback.clearIssue("obs:settings");
        feedback.success("OBS 设置已保存", "设置已保存在桌面执行端。点击测试 OBS 连接确认连接状态。");
      }
    } catch (failure) {
      if (!controller.signal.aborted) {
        const message = failure instanceof Error ? failure.message : "OBS 设置保存失败";
        setSettingsError(message);
        feedback.error("OBS 设置保存失败", message);
      }
    } finally {
      if (!controller.signal.aborted) { current.current = null; setBusy(false); }
    }
  }
  async function run(operation?: ObsOperation) {
    if (current.current || dirty) return;
    const controller = new AbortController();
    current.current = controller;
    setBusy(true);
    setError(null);
    try {
      const value = await (operation ? client.execute(operation, controller.signal) : client.getStatus(controller.signal));
      if (!controller.signal.aborted) {
        accept(value); feedback.clearIssue("obs:status");
        const confirmed = !operation || (value.connected && (operation.type === "set_scene" ? value.current_scene === operation.scene_name : operation.type === "start_recording" ? value.recording : !value.recording));
        if (!confirmed) feedback.error("OBS 操作未完成", "返回状态未确认本次操作，请刷新 OBS 状态后重试。");
        else feedback.success(operation?.type === "start_recording" ? "OBS 录制已开始" : operation?.type === "stop_recording" ? "OBS 录制已停止" : operation?.type === "set_scene" ? "OBS 场景已切换" : "OBS 状态已刷新", value.connected ? `当前场景：${value.current_scene}；${value.recording ? "正在录制" : "未录制"}。` : "OBS 控制未启用，请检查连接配置。");
      }
    } catch (failure) {
      if (!controller.signal.aborted) {
        setSnapshot(null);
        setError(`${failure instanceof Error ? failure.message : "OBS 操作失败"} 操作结果未知，请刷新状态确认。`);
        feedback.error("OBS 操作失败", `${failure instanceof Error ? failure.message : "OBS 操作失败"} 操作结果未知，请刷新状态确认。`);
      }
    } finally {
      if (!controller.signal.aborted) { current.current = null; setBusy(false); }
    }
  }
  const connected = snapshot?.connected === true;
  const controlsDisabled = busy || dirty;
  return <section className="panel obs-panel" aria-labelledby="obs-heading">
    <div className="section-title"><h2 id="obs-heading">OBS 场景与录制</h2><button disabled={controlsDisabled} onClick={() => void run()}>刷新 OBS 状态</button></div>
    <h3>OBS 连接设置</h3>
    <p className="field-hint">在运行桌面执行端的电脑打开 OBS，进入“工具 → WebSocket 服务器设置”，启用服务器，并将地址和密码填在这里。默认端口为 4455。</p>
    <fieldset disabled={busy || !settings || !settings.storage_available}>
      <label><input type="checkbox" checked={enabled} onChange={(event) => { setEnabled(event.target.checked); setSavedNotice(false); }} />启用 OBS 控制</label>
      <label htmlFor="obs-address">OBS WebSocket 地址</label>
      <input id="obs-address" type="text" value={address} maxLength={256} spellCheck={false} onChange={(event) => { setAddress(event.target.value); setSavedNotice(false); }} placeholder="ws://127.0.0.1:4455" />
      <p className="field-hint">保持默认地址即可连接同一台电脑上的 OBS；端口需与 OBS 内的设置一致。</p>
      <label htmlFor="obs-password">OBS WebSocket 密码</label>
      <input id="obs-password" type="password" value={password} maxLength={4096} autoComplete="new-password" disabled={clearPassword} onChange={(event) => { setPassword(event.target.value); setSavedNotice(false); }} />
      <p className="field-hint">{settings?.password_configured ? "已设置密码；留空保留现有密码。" : "尚未设置密码。"}</p>
      <label><input type="checkbox" checked={clearPassword} onChange={(event) => { setClearPassword(event.target.checked); if (event.target.checked) setPassword(""); setSavedNotice(false); }} />清除已保存密码</label>
    </fieldset>
    {!settings && <p className="field-hint">{busy ? "正在读取 OBS 设置…" : "请连接桌面执行端后重新读取设置。"}</p>}
    {settings && !settings.storage_available && <p className="field-hint">当前执行端无法保存设置，请通过项目启动器启动桌面执行端。</p>}
    {dirty && <p className="field-hint">有未保存的 OBS 设置，请先保存再测试连接。</p>}
    {savedNotice && <p role="status" className="field-hint">OBS 设置已保存，请测试连接。</p>}
    <div className="form-actions">
      <button className="primary-button" disabled={busy || !settings?.storage_available} onClick={() => void saveSettings()}>保存 OBS 设置</button>
      <button disabled={controlsDisabled || !settings} onClick={() => void run()}>测试 OBS 连接</button>
      <button disabled={busy || dirty} onClick={() => void reloadSettings()}>重新读取 OBS 设置</button>
      {dirty && <button disabled={busy} onClick={() => { if (settings) acceptSettings(settings); }}>放弃未保存的更改</button>}
    </div>
    {settingsError && <p role="alert" className="error-banner">{settingsError}</p>}
    <p className="field-hint">设置保存在已连接的桌面执行端，重启后仍会保留。保存设置与测试连接都不会开始录制。</p>
    <p className="muted">{snapshot ? (connected ? (snapshot.recording ? "正在录制" : "未录制") : "OBS 控制未启用") : "OBS 状态未知"}</p>
    {connected && <p className="muted">当前场景：{snapshot.current_scene}</p>}
    <label htmlFor="obs-scene">OBS 场景</label>
    <div className="inline-field-action"><select id="obs-scene" value={selected} disabled={controlsDisabled || !connected} onChange={(event) => setSelected(event.target.value)}>
      {!connected && <option value="">请连接并刷新 OBS</option>}
      {snapshot?.scenes.map((name) => <option key={name} value={name}>{name}</option>)}
    </select><button disabled={controlsDisabled || !connected || !selected || selected === snapshot?.current_scene} onClick={() => void run({ type: "set_scene", scene_name: selected })}>切换场景</button></div>
    <div className="form-actions"><button className="primary-button" disabled={controlsDisabled || !connected || snapshot?.recording} onClick={() => void run({ type: "start_recording" })}>开始录制</button>
      <button className="stop-button" disabled={controlsDisabled || !connected || !snapshot?.recording} onClick={() => void run({ type: "stop_recording" })}>停止录制</button></div>
    {error && <p role="alert" className="error-banner">{error}</p>}
    <p className="field-hint">停止播报不会停止录制。</p>
  </section>;
}
