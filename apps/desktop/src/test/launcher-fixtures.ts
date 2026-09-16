import type { LauncherSnapshot, LauncherServiceState } from "@meowlive/contracts";

export function launcherSnapshot(server: LauncherServiceState = "stopped", tts: LauncherServiceState = "stopped", windows: LauncherServiceState = "stopped"): LauncherSnapshot {
  return {
    schema_version: 1, session_token: "a".repeat(64),
    setup: { configuration_path: "./config/local/launcher.json", server_config: "./config/server.local.toml",
      llm_configured: true, llm_message: "LLM 配置已读取", windows_client_path: "./target/windows-client" },
    services: (["server", "tts", "windows"] as const).map((id) => {
      const state = id === "server" ? server : id === "tts" ? tts : windows;
      const managed = ["starting", "running", "stopping"].includes(state);
      return { id, state, managed, message: state === "failed" ? "启动失败，请检查日志" : "状态已读取",
        url: id === "windows" ? "ws://127.0.0.1:19600/ws/bridge" : `http://127.0.0.1:${id === "server" ? 19600 : 9880}`, log_path: `./logs/${id}.log`,
        can_start: ["stopped", "failed"].includes(state) && (id !== "windows" || ["running", "external"].includes(server)), can_stop: managed && state !== "stopping" };
    }),
  };
}
