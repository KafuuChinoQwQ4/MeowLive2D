/** Fixed dependency sources shared by browser setup links and the desktop IPC boundary. */
export const dependencySources = {
  docker: "https://docs.docker.com/desktop/setup/install/windows-install/",
  postgres: "https://www.postgresql.org/download/windows/",
  pgvector: "https://github.com/pgvector/pgvector#docker",
  neo4j: "https://neo4j.com/deployment-center/",
  project: "https://github.com/KafuuChinoQwQ4/MeowLive2D/archive/refs/heads/main.zip",
} as const;
export type DependencyId = keyof typeof dependencySources;

export function isDesktopApp() {
  return "__TAURI_INTERNALS__" in globalThis;
}

export async function openDependencyPage(id: DependencyId) {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_dependency_page", { id });
}
