import { workspacePages, type WorkspacePage } from "../navigation";
import { WorkspaceIcon } from "../WorkspaceIcon";
import { featureGuides } from "./content";

export function GuidePanel({ onNavigate }: { onNavigate: (page: WorkspacePage) => void }) {
  return <div className="guide-workspace">
    <section className="panel guide-start" aria-labelledby="guide-start-heading">
      <div className="section-title"><h2 id="guide-start-heading">新手入门</h2><span className="field-hint">从第一次播报开始</span></div>
      <ol className="guide-steps">
        <li><strong>打开应用</strong><p>Windows 安装版直接打开 App；源码版使用项目启动器。</p></li>
        <li><strong>准备语音</strong><p>自行准备 TTS 引擎和 GPT-SoVITS v2 模型，确认服务已就绪。</p></li>
        <li><strong>添加音色</strong><p>上传 3–10 秒录音，填写原文，设为当前音色。</p></li>
        <li><strong>试播一句</strong><p>到「语音播报」输入文字；需要角色动嘴时再连接 VTube Studio。</p></li>
      </ol>
      <button className="primary-button" onClick={() => onNavigate("setup")}>检查环境与模型<WorkspaceIcon name="arrow" /></button>
      <p className="field-hint">Windows 安装版可在“环境与模型”补齐 WSL2 语音后端；Docker 和数据库按需准备。需要观众存储时，到「环境与模型 → 数据库与可选功能」按步骤启用。</p>
    </section>
    <section aria-labelledby="guide-features-heading">
      <div className="section-title guide-section-title"><h2 id="guide-features-heading">功能使用</h2><span className="field-hint">展开查看步骤</span></div>
      <div className="guide-topics">{workspacePages.filter(page => page.id !== "guide").map(page => {
        const topic = featureGuides[page.id as keyof typeof featureGuides];
        return <details className="guide-topic" key={page.id}>
          <summary><WorkspaceIcon name={page.icon} /><span>{page.label}</span><WorkspaceIcon name="arrow" /></summary>
          <section className="guide-topic-body" aria-label={`${page.label}使用方法`}>
            <ol>{topic.steps.map(step => <li key={step}>{step}</li>)}</ol>
            {topic.note && <p className="guide-note">{topic.note}</p>}
            <button onClick={() => onNavigate(page.id)}>前往{page.label}<WorkspaceIcon name="arrow" /></button>
          </section>
        </details>;
      })}</div>
    </section>
  </div>;
}
