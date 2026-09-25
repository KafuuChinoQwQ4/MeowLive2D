import type { InstalledModel } from "@meowlive/contracts";

export function InstalledModels({ models, purpose, disabled, select, browse }: {
  models: InstalledModel[]; purpose: string; disabled: boolean;
  select: (id: string) => void; browse: (purpose: string) => void;
}) {
  return <>{(["tts", "asr"] as const).filter(kind => purpose === "all" || purpose === kind).map(kind => {
    const items = models.filter(model => model.purpose === kind);
    const asr = kind === "asr";
    return <section key={kind} aria-label={asr ? "语音识别模型" : "声音生成模型"}>
      <div className="model-section-heading"><div><h3>{asr ? "语音识别 · 训练文本提取" : "声音生成 · 播报与训练"}</h3>
        <p className="muted">{asr ? "上传训练录音后自动识别文字；选择对下一次转写生效。" : "切换时自动暂停 TTS 并释放旧模型，切换完成后自动启动。"}</p></div></div>
      {items.length ? <div className="model-local-list">{items.map(model => <article className="model-local-card" key={model.id}>
        <div><div className="model-card-title"><h3>{model.name}</h3>
          <span className={`model-badge ${model.ready ? "is-ready" : "is-pending"}`}>{model.selected ? "当前使用" : model.ready ? "可以使用" : "尚未就绪"}</span>
        </div><p>{model.message}</p><code className="model-path">{model.path}</code></div>
        <button className={model.ready && !model.selected ? "primary-button" : ""} disabled={disabled || !model.ready || model.selected}
          onClick={() => select(model.id)}>{model.selected ? (asr ? "当前转写模型" : "当前使用")
            : model.ready ? (asr ? "用于自动转写" : "选择此模型") : "尚未就绪"}</button>
      </article>)}</div> : <div className="model-empty"><h3>{asr ? "还没有找到语音识别模型" : "还没有找到语音模型"}</h3>
        <p>{asr ? "推荐 Whisper large-v3-turbo，下载后即可用于本地训练转写。" : "下载 GPT-SoVITS v2，并准备推理引擎。"}</p>
        <button className="primary-button" onClick={() => browse(kind)}>{asr ? "浏览语音识别模型" : "浏览声音生成模型"}</button></div>}
    </section>;
  })}</>;
}
