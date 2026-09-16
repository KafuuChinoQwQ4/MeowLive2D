import type { WorkspacePage } from "./navigation";

const paths: Record<WorkspacePage | "arrow" | "cat", string> = {
  overview: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  setup: "M3 4h18v12H3z M8 21h8 M12 16v5 M7 8l2 2-2 2 M12 12h5",
  speech: "M5 9v6 M9 5v14 M13 3v18 M17 7v10 M21 10v4",
  resources: "M4 20v-2a5 5 0 0 1 5-5h3a5 5 0 0 1 5 5v2 M14 6a4 4 0 1 1-8 0 4 4 0 0 1 8 0 M18 3l1 2 2 1-2 1-1 2-1-2-2-1 2-1z",
  agent: "M12 2v3 M9 2h6 M5 6h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z M8 11v2 M16 11v2 M8 16h8",
  live: "M8 5a9 9 0 0 0 0 14 M16 5a9 9 0 0 1 0 14 M10 8a5 5 0 0 0 0 8 M14 8a5 5 0 0 1 0 8 M12 11v2",
  obs: "M15 9l6-4v14l-6-4 M3 5h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z",
  training: "M4 19V9 M10 19V5 M16 19V12 M22 19V2 M2 22h20",
  arrow: "M5 12h14 M13 6l6 6-6 6",
  cat: "M4 11V3l6 4h4l6-4v8c2 7-1 10-8 10S2 18 4 11z M8 12v2 M16 12v2 M10 17l2 1 2-1",
};
export function WorkspaceIcon({ name, className }: { name: keyof typeof paths; className?: string }) {
  return <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
