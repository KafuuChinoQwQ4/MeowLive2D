import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import type { ModelLibraryClient } from "../../services/model-library";
import { modelLibrarySnapshot } from "../../test/model-library-fixtures";
import { ModelLibraryPanel } from "./ModelLibraryPanel";

function client(): ModelLibraryClient {
  return { getStatus: vi.fn().mockResolvedValue(modelLibrarySnapshot()), scan: vi.fn().mockResolvedValue(modelLibrarySnapshot()), select: vi.fn().mockResolvedValue(modelLibrarySnapshot()), download: vi.fn().mockResolvedValue(modelLibrarySnapshot()), cancel: vi.fn().mockResolvedValue(modelLibrarySnapshot()) };
}
it("shows environment and local model without requiring the main service", async () => {
  const api = client();
  render(<ModelLibraryPanel client={api} token={"a".repeat(64)} onSelected={vi.fn()} />);
  expect(await screen.findByText("WSL2 环境可用")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "当前使用" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "重新扫描" }));
  expect(api.scan).toHaveBeenCalledWith("a".repeat(64), expect.any(AbortSignal));
});
it("paginates catalog and keeps unsupported engines explicit", async () => {
  render(<ModelLibraryPanel client={client()} token={"a".repeat(64)} onSelected={vi.fn()} />);
  await screen.findByText("WSL2 环境可用");
  await userEvent.click(screen.getByRole("tab", { name: /下载模型/ }));
  const catalog = screen.getByRole("region", { name: "可下载模型" });
  expect(within(catalog).getAllByRole("article")).toHaveLength(4);
  expect(screen.queryByRole("heading", { name: "语音模型 5" })).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "下一页" }));
  expect(screen.getByRole("heading", { name: "语音模型 5" })).toBeInTheDocument();
  await userEvent.type(screen.getByRole("searchbox", { name: "搜索语音模型" }), "GPT");
  expect(within(catalog).getAllByRole("article")).toHaveLength(1);
  expect(screen.getByRole("heading", { name: "GPT-SoVITS v2" })).toBeInTheDocument();
});
it("blocks writes while the launcher token is stale", async () => {
  render(<ModelLibraryPanel client={client()} token={null} onSelected={vi.fn()} />);
  await screen.findByText("WSL2 环境可用");
  expect(screen.getByRole("button", { name: "重新扫描" })).toBeDisabled();
  await userEvent.click(screen.getByRole("tab", { name: /下载模型/ }));
  expect(screen.getAllByRole("button", { name: /下载权重/ }).every(button => (button as HTMLButtonElement).disabled)).toBe(true);
});
it("refreshes launcher readiness after selecting an installed model", async () => {
  const api = client();
  vi.mocked(api.getStatus).mockResolvedValue(modelLibrarySnapshot({ selected_id: null, installed: [{ ...modelLibrarySnapshot().installed[0], selected: false }] }));
  const onSelected = vi.fn();
  render(<ModelLibraryPanel client={api} token={"a".repeat(64)} onSelected={onSelected} />);
  await userEvent.click(await screen.findByRole("button", { name: "选择此模型" }));
  await waitFor(() => expect(onSelected).toHaveBeenCalledTimes(1));
  expect(api.select).toHaveBeenCalledWith("local-gpt", "a".repeat(64), expect.any(AbortSignal));
});
it("shows upgrade instructions and blocks model actions on WSL1", async () => {
  const api = client();
  vi.mocked(api.getStatus).mockResolvedValue(modelLibrarySnapshot({ environment: { kind: "wsl1", distro: "Ubuntu", release: "Microsoft", ready: false, message: "请先升级到 WSL2" } }));
  render(<ModelLibraryPanel client={api} token={"a".repeat(64)} onSelected={vi.fn()} />);
  expect(await screen.findByText("请先升级到 WSL2")).toBeInTheDocument();
  expect(screen.getByText("请先完成 Windows 环境准备")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "重新扫描" })).toBeDisabled();
});

it("cancels the exact download job and reports progress", async () => {
  const api = client();
  vi.mocked(api.getStatus).mockResolvedValue(modelLibrarySnapshot({ downloads: [{ id: "job-7", model_id: "gpt-sovits-v2", state: "downloading", message: "正在下载权重", path: "./data/models/gpt", downloaded_bytes: 1048576, total_bytes: 2097152 }] }));
  render(<ModelLibraryPanel client={api} token={"a".repeat(64)} onSelected={vi.fn()} />);
  await userEvent.click(await screen.findByRole("tab", { name: /下载任务/ }));
  expect(screen.getByRole("progressbar")).toHaveAttribute("value", "1048576");
  await userEvent.click(screen.getByRole("button", { name: "取消下载" }));
  expect(api.cancel).toHaveBeenCalledWith("job-7", "a".repeat(64), expect.any(AbortSignal));
});
