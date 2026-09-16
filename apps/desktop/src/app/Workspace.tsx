import { useEffect, useRef, type ReactNode } from "react";
import { workspacePages, type WorkspacePage } from "./navigation";
import { WorkspaceIcon } from "./WorkspaceIcon";
import { useWorkspaceNavigation } from "./useWorkspaceNavigation";

export interface WorkspaceProps {
  overview: ReactNode;
  pages: Record<Exclude<WorkspacePage, "overview" | "setup">, ReactNode>;
  setup: ReactNode;
  ready: boolean;
  status: { label: string; available: boolean | null }[];
  notice?: ReactNode;
}

export function Workspace({ overview, pages, setup, ready, status, notice }: WorkspaceProps) {
  const { active, visited, navigate } = useWorkspaceNavigation();
  const page = workspacePages.find(item => item.id === active)!;
  const content = useRef<HTMLElement>(null);
  const title = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (content.current) content.current.scrollTop = 0;
    const frame = requestAnimationFrame(() => {
      if (window.location.hash === "#history-heading") document.getElementById("history-heading")?.scrollIntoView?.({ block: "start" });
    });
    return () => cancelAnimationFrame(frame);
  }, [active, ready]);
  return <div className="studio-shell">
    <a className="skip-navigation" href="#studio-main" onClick={event => { event.preventDefault(); content.current?.focus(); }}>跳到功能内容</a>
    <aside className="studio-sidebar">
      <a className="studio-brand" href="#overview" onClick={event => { event.preventDefault(); navigate("overview"); }} aria-label="MeowLive2D 首页">
        <span className="studio-brand-icon"><WorkspaceIcon name="cat" /></span><span>MeowLive2D<small>你的虚拟直播工作室</small></span>
      </a>
      <nav className="studio-navigation" aria-label="功能导航">
        {workspacePages.map((item, index) => <div className="studio-nav-entry" key={item.id}>
          {(index === 0 || workspacePages[index - 1].group !== item.group) && <p className="studio-nav-group">{item.group}</p>}
          <a href={`#${item.id}`} className="studio-nav-link" aria-current={active === item.id ? "page" : undefined}
            onClick={event => {
              if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
              event.preventDefault(); navigate(item.id);
            }}><WorkspaceIcon name={item.icon} /><span>{item.label}</span><span className="studio-nav-dot" aria-hidden="true" /></a>
        </div>)}
      </nav>
      <div className="studio-sidebar-footer"><span className="studio-footer-symbol">✦</span><p>为每个角色<br /><strong>留一个发声的地方。</strong></p><span className="studio-edition">MEOWLIVE STUDIO · 01</span></div>
    </aside>
    <div className="studio-body">
      <header className="studio-topbar"><div className="studio-breadcrumb">工作室 <span>/</span> <strong>{page.label}</strong></div>
        <div className="studio-system-status" aria-label="系统运行状态">{status.map(item => <span key={item.label} className={`studio-status ${item.available === null ? "neutral" : item.available ? "online" : "offline"}`}><i />{item.label}</span>)}</div>
      </header>
      <main id="studio-main" className="studio-content" ref={content} tabIndex={-1}>
        <div className="studio-page-header"><div><p className="studio-page-kicker">{page.group}</p><h1 ref={title}>{page.title}</h1><p>{page.description}</p></div><span className="studio-page-number">0{workspacePages.indexOf(page) + 1}</span></div>
        <section className="studio-page" aria-label="运行总览" hidden={active !== "overview"}>
          <div className="studio-welcome"><div className="studio-welcome-copy"><span className="studio-welcome-label">YOUR STAGE, YOUR VOICE</span><h2>让角色，<br />拥有自己的声音。</h2><p>从一句问候开始，让每一次互动都鲜活起来。</p><button type="button" onClick={() => navigate("speech")}>开始一次播报 <WorkspaceIcon name="arrow" /></button></div>
            <div className="studio-welcome-art" aria-hidden="true"><div className="studio-orbit orbit-one" /><div className="studio-orbit orbit-two" /><div className="studio-avatar"><WorkspaceIcon name="cat" /></div><div className="studio-wave">{[12, 24, 36, 20, 46, 64, 38, 78, 54, 30, 60, 42, 22, 34, 16].map((height, i) => <span style={{ height }} key={i} />)}</div><span className="studio-art-caption">a little voice, a live world.</span></div>
          </div>
          <button className="studio-setup-link" onClick={() => navigate("setup")}><WorkspaceIcon name="setup" /><span><strong>第一次使用？先检查环境与模型</strong><small>检测 WSL2、选择本地模型，或下载新的声音</small></span><WorkspaceIcon name="arrow" /></button>
          {overview}
          <div className="studio-shortcuts" aria-label="常用功能">
            {(["speech", "resources", "agent"] as const).map(id => <button className="studio-shortcut" key={id} onClick={() => navigate(id)}><WorkspaceIcon name={id} /><span><strong>{id === "speech" ? "试播一句话" : id === "resources" ? "准备角色与音色" : "开启自然互动"}</strong><small>{id === "speech" ? "文字输入，即刻表达" : id === "resources" ? "让声音和形象更合拍" : "让角色回应每份关注"}</small></span><WorkspaceIcon name="arrow" /></button>)}
          </div>
        </section>
        <section className="studio-page studio-page-setup" aria-label="环境与模型" hidden={active !== "setup"}>{visited.has("setup") && setup}</section>
        {active !== "overview" && active !== "setup" && !ready && <section className="panel studio-unavailable"><WorkspaceIcon name="overview" /><h2>先准备好主服务</h2><p>主服务尚未就绪。前往启动页面打开开关，运行后即可使用这些功能。</p><button className="primary-button" onClick={() => navigate("overview")}>前往启动与运行</button></section>}
        {ready && workspacePages.filter(item => item.id !== "overview" && item.id !== "setup").map(item => {
          const id = item.id as Exclude<WorkspacePage, "overview" | "setup">;
          return <section key={id} className={`studio-page studio-page-${id}`} aria-label={item.label} hidden={active !== id}>
            {visited.has(id) && <>{notice}{pages[id]}</>}
          </section>;
        })}
        <footer className="studio-content-footer"><span>MeowLive2D</span><span>给声音一点个性，给互动一点温度。</span></footer>
      </main>
    </div>
  </div>;
}
