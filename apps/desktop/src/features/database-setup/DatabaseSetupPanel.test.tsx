import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { DatabaseSetupPanel } from "./DatabaseSetupPanel";

afterEach(() => { vi.unstubAllGlobals(); vi.resetAllMocks(); });

it("opens the selected official source through the desktop boundary without exposing a URL command", async () => {
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  vi.mocked(invoke).mockResolvedValue(undefined);
  render(<DatabaseSetupPanel />);
  expect(invoke).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("link", { name: "下载 Docker Desktop（Windows）" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("open_dependency_page", { id: "docker" }));
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("keeps the official address available when the native browser command fails", async () => {
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  vi.mocked(invoke).mockRejectedValue(new Error("unavailable"));
  render(<DatabaseSetupPanel />);
  await userEvent.click(screen.getByRole("link", { name: "查看 pgvector 官方镜像与安装说明" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("https://github.com/pgvector/pgvector#docker");
  expect(screen.getByRole("link", { name: "查看 pgvector 官方镜像与安装说明" })).toHaveAttribute("href", "https://github.com/pgvector/pgvector#docker");
});

it("lets users expand the Windows setup instructions without a running server", async () => {
  render(<DatabaseSetupPanel />);
  const summary = screen.getByText("Windows 安装版：补齐数据库并启用观众功能", { selector: "summary" });
  expect(summary.closest("details")).not.toHaveAttribute("open");
  await userEvent.click(summary);
  expect(summary.closest("details")).toHaveAttribute("open");
  expect(screen.getByRole("link", { name: "下载项目配置与源码 ZIP" })).toBeVisible();
  expect(invoke).not.toHaveBeenCalled();
});
