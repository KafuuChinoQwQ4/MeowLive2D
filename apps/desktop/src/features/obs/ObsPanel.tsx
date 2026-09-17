import { useEffect, useRef, useState } from "react";
import type { ObsOperation, ObsSnapshot } from "@meowlive/contracts";
import { type ObsClient } from "../../services/server/obs";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export function ObsPanel({ client }: { client: ObsClient }) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<ObsSnapshot | null>(null);
  const [selected, setSelected] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const current = useRef<AbortController | null>(null);
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
    void client.getStatus(controller.signal).then((value) => {
      if (!controller.signal.aborted) { accept(value); feedback.clearIssue("obs:status"); }
    }).catch((failure: unknown) => {
      if (!controller.signal.aborted) { setError(failure instanceof Error ? failure.message : "OBS 状态读取失败"); feedback.reportIssue("obs:status", "OBS 状态读取失败", failure); }
    }).finally(() => {
      if (!controller.signal.aborted) { current.current = null; setBusy(false); }
    });
    return () => { controller.abort(); current.current?.abort(); };
  }, [client, feedback]);
  async function run(operation?: ObsOperation) {
    if (current.current) return;
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
  return <section className="panel obs-panel" aria-labelledby="obs-heading">
    <div className="section-title"><h2 id="obs-heading">OBS 场景与录制</h2><button disabled={busy} onClick={() => void run()}>刷新 OBS 状态</button></div>
    <p className="muted">{snapshot ? (connected ? (snapshot.recording ? "正在录制" : "未录制") : "OBS 控制未启用") : "OBS 状态未知"}</p>
    {connected && <p className="muted">当前场景：{snapshot.current_scene}</p>}
    <label htmlFor="obs-scene">OBS 场景</label>
    <div className="inline-field-action"><select id="obs-scene" value={selected} disabled={busy || !connected} onChange={(event) => setSelected(event.target.value)}>
      {!connected && <option value="">请连接并刷新 OBS</option>}
      {snapshot?.scenes.map((name) => <option key={name} value={name}>{name}</option>)}
    </select><button disabled={busy || !connected || !selected || selected === snapshot?.current_scene} onClick={() => void run({ type: "set_scene", scene_name: selected })}>切换场景</button></div>
    <div className="form-actions"><button className="primary-button" disabled={busy || !connected || snapshot?.recording} onClick={() => void run({ type: "start_recording" })}>开始录制</button>
      <button className="stop-button" disabled={busy || !connected || !snapshot?.recording} onClick={() => void run({ type: "stop_recording" })}>停止录制</button></div>
    {error && <p role="alert" className="error-banner">{error}</p>}
    <p className="field-hint">录制由上述按钮控制。立即停止播报不会停止 OBS 录制。</p>
  </section>;
}
