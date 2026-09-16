import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useLayoutEffect } from "react";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "./App";
import { useWorkspaceNavigation } from "./useWorkspaceNavigation";
import { jsonResponse, serverStatus, speech, deferred } from "../test/server-fixtures";
import { agentStatus } from "../test/agent-fixtures";

afterEach(() => { vi.unstubAllGlobals(); window.history.replaceState(null, "", "/"); });

it("switching away during a speech submission does not cancel or replay it", async () => {
  const response = deferred<Response>();
  const signals: AbortSignal[] = [];
  const queued = speech({ text: "切页时保留在途播报" });
  let accepted = false;
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async (url, init) => {
    if (String(url).endsWith("/api/speech")) {
      signals.push(init!.signal!);
      const result = await response.promise;
      accepted = true;
      return result;
    }
    if (String(url).endsWith("/api/agent")) return jsonResponse(agentStatus());
    // A status poll after acceptance must include the task the server just queued.
    return jsonResponse(serverStatus({ speeches: accepted ? [queued] : [] }));
  }));
  render(<App />);
  await userEvent.click(await screen.findByRole("link", { name: "语音播报" }));
  await userEvent.type(await screen.findByRole("textbox", { name: "播报文本" }), "切页时保留在途播报");
  await userEvent.click(screen.getByRole("button", { name: "加入播报队列" }));
  await waitFor(() => expect(signals).toHaveLength(1));
  await userEvent.click(screen.getByRole("link", { name: "Agent 互动" }));
  expect(signals[0].aborted).toBe(false);
  await act(async () => { response.resolve(jsonResponse(queued)); });
  await userEvent.click(screen.getByRole("link", { name: "语音播报" }));
  expect(await screen.findByText("切页时保留在途播报")).toBeInTheDocument();
  expect(screen.getByRole("textbox", { name: "播报文本" })).toHaveValue("");
  expect(signals).toHaveLength(1);
});

it("a speech-history deep link selects the speech page and unknown hashes select overview", async () => {
  vi.stubGlobal("fetch", vi.fn<typeof fetch>().mockImplementation(async () => jsonResponse(serverStatus())));
  window.history.replaceState(null, "", "#history-heading");
  render(<App />);
  expect(await screen.findByRole("heading", { name: "播报记录" })).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "语音播报" })).toHaveAttribute("aria-current", "page");
  await act(async () => { window.history.replaceState(null, "", "#unknown"); window.dispatchEvent(new HashChangeEvent("hashchange")); });
  expect(await screen.findByRole("heading", { name: "运行总览" })).toBeInTheDocument();
  expect(screen.queryByRole("heading", { name: "播报记录" })).not.toBeInTheDocument();
});

it("accepts a hash change between the initial render and event subscription", () => {
  window.history.replaceState(null, "", "#history-heading");
  function NavigationDuringMount() {
    const { active, visited } = useWorkspaceNavigation();
    useLayoutEffect(() => {
      window.history.replaceState(null, "", "#unknown");
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    }, []);
    return <output>{active}: {Array.from(visited).join(", ")}</output>;
  }
  render(<NavigationDuringMount />);
  expect(screen.getByRole("status")).toHaveTextContent("overview: speech, overview");
});
