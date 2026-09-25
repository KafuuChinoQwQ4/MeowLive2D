import { createContext, useEffect, useRef, type ReactNode } from "react";
import { workspacePages, type WorkspacePage } from "./navigation";
import { WorkspaceIcon } from "./WorkspaceIcon";
import { useWorkspaceNavigation } from "./useWorkspaceNavigation";
import { FeedbackScope } from "./feedback/OperationFeedback";
import { AdminGate } from "./AdminGate";
import type { AdminSessionClient } from "../services/server/auth";
import { GuidePanel } from "./guide/GuidePanel";
import { DatabaseSetupPanel } from "../features/database-setup/DatabaseSetupPanel";

export interface WorkspaceProps {
  overview: ReactNode;
  pages: Record<Exclude<WorkspacePage, "overview" | "setup" | "guide">, ReactNode>;
  setup: ReactNode;
  ready: boolean;
  status: { label: string; available: boolean | null }[];
  notice?: ReactNode;
  adminClient?: AdminSessionClient;
}

export const WorkspaceActiveContext = createContext<WorkspacePage | null>(null);

export function Workspace({ overview, pages, setup, ready, status, notice, adminClient }: WorkspaceProps) {
  const { active, visited, navigate } = useWorkspaceNavigation();
  const page = workspacePages.find(item => item.id === active)!;
  const content = useRef<HTMLElement>(null);
  useEffect(() => {
    if (content.current) content.current.scrollTop = 0;
    const frame = requestAnimationFrame(() => {
      if (window.location.hash === "#history-heading") document.getElementById("history-heading")?.scrollIntoView?.({ block: "start" });
    });
    return () => cancelAnimationFrame(frame);
  }, [active, ready]);
  return <WorkspaceActiveContext.Provider value={active}><div className="studio-shell">
    <a className="skip-navigation" href="#studio-main" onClick={event => { event.preventDefault(); content.current?.focus(); }}>跳到功能内容</a>
    <aside className="studio-sidebar">
      <a className="studio-brand" href="#overview" onClick={event => { event.preventDefault(); navigate("overview"); }} aria-label="MeowLive2D 首页">
        <span className="studio-brand-icon"><WorkspaceIcon name="cat" /></span><span>MeowLive2D<small>虚拟直播工作室</small></span>
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
    </aside>
    <div className="studio-body">
      <header className="studio-topbar"><div className="studio-breadcrumb">工作室 <span>/</span> <strong>{page.label}</strong></div>
        <div className="studio-system-status" aria-label="系统运行状态">{status.map(item => <span key={item.label} className={`studio-status ${item.available === null ? "neutral" : item.available ? "online" : "offline"}`}><i />{item.label}</span>)}</div>
      </header>
      <main id="studio-main" className="studio-content" ref={content} tabIndex={-1}>
        <div className="studio-page-header"><div><h1>{page.title}</h1><p>{page.description}</p></div>{active !== "guide" && <button className="studio-help-button" onClick={() => navigate("guide")}><WorkspaceIcon name="guide" />使用帮助</button>}</div>
        <section className="studio-page" aria-label="运行总览" hidden={active !== "overview"}>
          {overview}
          <div className="studio-shortcuts" aria-label="常用功能">
            {(["setup", "resources", "speech"] as const).map(id => <button className="studio-shortcut" key={id} onClick={() => navigate(id)}><WorkspaceIcon name={id} /><span>{id === "setup" ? "环境与模型" : id === "speech" ? "试播一句话" : "角色与人物卡"}</span><WorkspaceIcon name="arrow" /></button>)}
          </div>
        </section>
        <section className="studio-page studio-page-setup" aria-label="环境与模型" hidden={active !== "setup"}>{visited.has("setup") && <FeedbackScope name="setup"><DatabaseSetupPanel />{setup}</FeedbackScope>}</section>
        <section className="studio-page studio-page-guide" aria-label="使用指南" hidden={active !== "guide"}>{visited.has("guide") && <GuidePanel onNavigate={navigate} />}</section>
        {active !== "overview" && active !== "setup" && active !== "guide" && !ready && <section className="panel studio-unavailable"><WorkspaceIcon name="overview" /><h2>正在等待主服务</h2><p>主服务自动就绪后即可使用；启动异常可在运行页查看。</p><button className="primary-button" onClick={() => navigate("overview")}>前往启动与运行</button></section>}
        {ready && workspacePages.filter(item => item.id !== "overview" && item.id !== "setup" && item.id !== "guide").map(item => {
          const id = item.id as keyof WorkspaceProps["pages"];
          return <section key={id} className={`studio-page studio-page-${id}`} aria-label={item.label} hidden={active !== id}>
            {visited.has(id) && (adminClient
              ? <AdminGate client={adminClient}><FeedbackScope name={id}>{notice}{pages[id]}</FeedbackScope></AdminGate>
              : <FeedbackScope name={id}>{notice}{pages[id]}</FeedbackScope>)}
          </section>;
        })}
      </main>
    </div>
  </div></WorkspaceActiveContext.Provider>;
}
