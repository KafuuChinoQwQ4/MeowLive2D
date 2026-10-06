import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
vi.mock("../services/desktop/window", () => ({ hasNativeWindow: () => true, controlWindow: vi.fn().mockResolvedValue(undefined) }));
import { controlWindow } from "../services/desktop/window";
import { WindowChrome } from "./WindowChrome";
it("routes custom window buttons to native window controls", async () => {
  render(<WindowChrome />);
  await userEvent.click(screen.getByRole("button", { name: "最小化窗口" }));
  expect(controlWindow).toHaveBeenLastCalledWith("minimize");
  await userEvent.click(screen.getByRole("button", { name: "最大化或还原窗口" }));
  expect(controlWindow).toHaveBeenLastCalledWith("toggleMaximize");
  await userEvent.click(screen.getByRole("button", { name: "关闭窗口" }));
  expect(controlWindow).toHaveBeenLastCalledWith("close");
});
