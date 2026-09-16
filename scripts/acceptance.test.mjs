import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import http from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { collectAcceptance, parseArgs } from "./acceptance.mjs";

const snapshots = {
  "/api/status": { protocol_version: 3, session_id: "session-a", bridge_connected: true, generation: 1, speeches: [] },
  "/api/agent": { paused: false, phase: "waiting", settings: { persona: "SECRET PERSONA", topic: "secret" }, events: [], last_error: null, current_speech_id: null, llm_configured: true, bridge_connected: true },
  "/api/live": { platform: "controlled", configured: true, phase: "connected", room_id: "private-room", accepted_events: 0, duplicate_events: 0, rejected_events: 0, reconnect_attempts: 0, last_error: null },
  "/api/training": { enabled: true, busy: false, jobs: [], versions: [] },
  "/api/runtime/preset": { mode: "cloud", model: "private-model", local_only: false, verified: false, max_tokens: 64, timeout_seconds: 5, measurement: null, message: "private path /home/test" },
};

test("explicit observe mode is equivalent to the read-only default", () => {
  const now = new Date("2026-09-15T00:00:00Z");
  assert.deepEqual(parseArgs(["--observe"], now), parseArgs([], now));
});

async function serve(t, handler) {
  const methods = [];
  const server = http.createServer((request, response) => {
    methods.push(request.method);
    handler(request, response);
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise(resolve => {
    server.close(resolve);
    server.closeAllConnections();
  }));
  return { baseUrl: `http://127.0.0.1:${server.address().port}`, methods };
}

function json(response, status, body) {
  response.writeHead(status, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

async function run(t, handler, options = {}) {
  const server = await serve(t, handler);
  const directory = await mkdtemp(path.join(tmpdir(), "meowlive-acceptance-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const output = path.join(directory, "evidence.json");
  const report = await collectAcceptance({
    baseUrl: server.baseUrl,
    durationMs: 180,
    intervalMs: 50,
    requestTimeoutMs: 80,
    output,
    execution: "simulated",
    includeRuntimePreset: true,
    ...options,
  });
  return { ...server, output, report, saved: JSON.parse(await readFile(output, "utf8")) };
}

test("samples successful endpoints with GET and writes a manual acceptance report", async t => {
  const result = await run(t, (request, response) => json(response, 200, snapshots[request.url]));
  assert.deepEqual(new Set(result.methods), new Set(["GET"]));
  assert.equal(result.report.overall_acceptance, "manual_required");
  assert.equal(result.saved.execution, "simulated");
  assert.ok(result.saved.samples.length >= 2);
  assert.ok(result.saved.samples.every(sample => sample.offset_ms >= 0));
  assert.ok(result.saved.summary.api_query_ms.p50 !== null);
  assert.equal(result.saved.summary.resource_metrics.process_memory_mib, null);
});

test("records bounded timeout and HTTP/JSON errors without raw bodies", async t => {
  const result = await run(t, (request, response) => {
    if (request.url === "/api/status") return setTimeout(() => json(response, 200, snapshots[request.url]), 150);
    if (request.url === "/api/agent") return json(response, 503, { error: "token=secret", reply: "full answer" });
    if (request.url === "/api/live") return response.end("not json: /home/private/key");
    json(response, 200, snapshots[request.url]);
  });
  const serialized = JSON.stringify(result.saved);
  assert.match(serialized, /request_timeout/);
  assert.match(serialized, /http_503/);
  assert.match(serialized, /invalid_json/);
  assert.doesNotMatch(serialized, /token=secret|full answer|\/home\/private/);
});

test("sanitizes private text, replies, arbitrary errors, rooms, paths and model names", async t => {
  const result = await run(t, (request, response) => {
    const body = structuredClone(snapshots[request.url]);
    if (request.url === "/api/status") body.speeches = [{ id: "speech-new", status: "failed", text: "PRIVATE REPLY", voice_id: "/private/voice", error: "API KEY abc" }];
    if (request.url === "/api/agent") body.events = [{ event: { id: "event-new", viewer: "PRIVATE VIEWER", kind: { type: "chat", text: "PRIVATE CHAT" } }, status: "failed", speech_id: "speech-new", error: "raw provider error" }];
    json(response, 200, body);
  });
  const serialized = JSON.stringify(result.saved);
  assert.doesNotMatch(serialized, /SECRET PERSONA|PRIVATE|private-room|private-model|raw provider|API KEY/);
  assert.match(serialized, /speech-new/);
  assert.match(serialized, /event-new/);
});

test("excludes baseline history while counting short-lived new terminal tasks", async t => {
  let statusReads = 0;
  const result = await run(t, (request, response) => {
    const body = structuredClone(snapshots[request.url]);
    if (request.url === "/api/status") {
      statusReads += 1;
      body.speeches = statusReads === 1
        ? [{ id: "old", status: "completed", text: "old" }]
        : statusReads === 2
          ? [{ id: "short", status: "completed", text: "short" }, { id: "old", status: "completed", text: "old" }]
          : [{ id: "old", status: "completed", text: "old" }];
    }
    json(response, 200, body);
  }, { durationMs: 220 });
  assert.deepEqual(result.report.summary.new_tasks.speeches, { completed: 1, failed: 0, cancelled: 0, unknown: 0 });
  assert.equal(result.report.summary.counts_are_lower_bounds, true, "a shrinking history is only an observed lower bound");
});

test("marks counts as an observation lower bound after service restart and counts interruptions", async t => {
  let statusReads = 0;
  const result = await run(t, (request, response) => {
    const body = structuredClone(snapshots[request.url]);
    if (request.url === "/api/status") {
      statusReads += 1;
      body.session_id = statusReads < 3 ? "session-a" : "session-b";
      body.bridge_connected = statusReads < 2;
    }
    json(response, 200, body);
  }, { durationMs: 220 });
  assert.equal(result.report.summary.service_restarts, 1);
  assert.ok(result.report.summary.connection_interruptions >= 1);
  assert.equal(result.report.summary.counts_are_lower_bounds, true);
});

test("rejects credential or query-bearing origins and existing output files", async t => {
  assert.throws(() => parseArgs(["--base-url", "http://user:pass@127.0.0.1:1"]), /credentials/i);
  assert.throws(() => parseArgs(["--base-url", "http://127.0.0.1:1?token=x"]), /query/i);
  assert.throws(() => parseArgs(["--duration-seconds", "0"]), /1\.\.3600/);
  assert.throws(() => parseArgs(["--interval-ms", "99"]), /100\.\.10000/);
  const directory = await mkdtemp(path.join(tmpdir(), "meowlive-acceptance-existing-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const output = path.join(directory, "exists.json");
  await writeFile(output, "keep");
  const server = await serve(t, (request, response) => json(response, 200, snapshots[request.url]));
  await assert.rejects(() => collectAcceptance({ baseUrl: server.baseUrl, durationMs: 20, intervalMs: 10, requestTimeoutMs: 20, output, execution: "unknown", includeRuntimePreset: false }), /already exists/i);
  assert.equal(await readFile(output, "utf8"), "keep");
});

test("rejects responses larger than one MiB", async t => {
  const oversized = JSON.stringify({ padding: "x".repeat(1024 * 1024) });
  const result = await run(t, (request, response) => {
    if (request.url === "/api/status") return response.end(oversized);
    json(response, 200, snapshots[request.url]);
  }, { durationMs: 90 });
  assert.match(JSON.stringify(result.saved), /response_too_large/);
});
