import { useEffect, useRef, useState } from "react";
import type { ServerClient } from "../../services/server";
import { useFeedback } from "../../app/feedback/OperationFeedback";

export function SpeechSettingsPanel({ client }: { client: ServerClient }) {
  const feedback = useFeedback();
  const [value, setValue] = useState(4);
  const [saved, setSaved] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reload, setReload] = useState(0);
  const action = useRef<AbortController | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    setSaved(null);
    setError(null);
    setPending(false);
    void client.getSpeechSettings(controller.signal).then(settings => {
      if (!controller.signal.aborted) {
        setValue(settings.sentence_batch_size);
        setSaved(settings.sentence_batch_size);
      }
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : "读取合成设置失败。");
    });
    return () => { controller.abort(); action.current?.abort(); action.current = null; };
  }, [client, reload]);

  async function save() {
    if (action.current || saved === null) return;
    const controller = new AbortController();
    action.current = controller;
    setPending(true);
    setError(null);
    try {
      const settings = await client.saveSpeechSettings({ sentence_batch_size: value }, controller.signal);
      if (!controller.signal.aborted) {
        setValue(settings.sentence_batch_size);
        setSaved(settings.sentence_batch_size);
        feedback.success("合成设置已保存", "后续开始合成的播报将使用新的分句数。");
      }
    } catch (reason) {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : "保存合成设置失败。");
    } finally {
      if (action.current === controller) { action.current = null; setPending(false); }
    }
  }

  return <section className="panel" aria-labelledby="speech-settings-heading">
    <h2 id="speech-settings-heading">合成设置</h2>
    <label htmlFor="speech-sentence-batch-size">并行合成分句数</label>
    <select id="speech-sentence-batch-size" value={value} disabled={saved === null || pending}
      aria-describedby="speech-batch-hint" onChange={event => setValue(Number(event.target.value))}>
      {Array.from({ length: 16 }, (_, i) => i + 1).map(count => <option key={count} value={count}>{count}</option>)}
    </select>
    <p id="speech-batch-hint" className="field-hint">同一条播报内最多同时合成的分句数，默认 4。数值越大，显存与计算压力越高。不同播报仍依次播放。</p>
    <p className="field-hint">保存后对后续开始合成的播报生效，无需重启；已开始的合成不受影响。</p>
    {error && <p className="field-error">{error}</p>}
    <div className="form-actions">
      <button type="button" className="primary-button" disabled={saved === null || pending || value === saved}
        onClick={() => { void save(); }}>{pending ? "正在保存…" : "保存合成设置"}</button>
      {error && saved === null && <button type="button" onClick={() => setReload(current => current + 1)}>重试读取设置</button>}
    </div>
  </section>;
}
