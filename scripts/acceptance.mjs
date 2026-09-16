#!/usr/bin/env node

import { mkdir, open } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const MAX_RESPONSE_BYTES = 1024 * 1024;
const DEFAULT_BASE_URL = "http://127.0.0.1:19600";
const ENDPOINTS = [
  ["status", "/api/status"],
  ["agent", "/api/agent"],
  ["live", "/api/live"],
  ["training", "/api/training"],
];
const ACTIVE_SPEECH = new Set(["queued", "synthesizing", "ready", "playing"]);

function usage() {
  return `Usage: node scripts/acceptance.mjs [options]

Read-only observation of a running MeowLive2D server.

Options:
  --observe                    Explicit read-only mode (also the default)
  --base-url <origin>          HTTP(S) origin (default: ${DEFAULT_BASE_URL})
  --duration-seconds <1..3600> Observation duration (default: 60)
  --interval-ms <100..10000>   Sampling interval (default: 1000)
  --output <path>              New JSON evidence file (default: data/acceptance-<timestamp>.json)
  --execution <value>          simulated, physical, or unknown (default: unknown)
  --include-runtime-preset     Also read GET /api/runtime/preset
  --help                       Show this help

This command sends GET requests only. Its report always requires manual acceptance review.`;
}

function integer(value, flag, minimum, maximum) {
  if (!/^\d+$/.test(value ?? "")) throw new Error(`${flag} must be an integer in ${minimum}..${maximum}`);
  const parsed = Number(value);
  if (parsed < minimum || parsed > maximum) throw new Error(`${flag} must be in ${minimum}..${maximum}`);
  return parsed;
}

function cleanOrigin(value) {
  let url;
  try { url = new URL(value); } catch { throw new Error("--base-url must be a valid HTTP(S) origin"); }
  if (!['http:', 'https:'].includes(url.protocol)) throw new Error("--base-url must use HTTP or HTTPS");
  if (url.username || url.password) throw new Error("--base-url must not contain credentials");
  if (url.search) throw new Error("--base-url must not contain a query");
  if (url.hash) throw new Error("--base-url must not contain a fragment");
  if (url.pathname !== "/") throw new Error("--base-url must be an origin without a path");
  return url.origin;
}

function defaultOutput(now = new Date()) {
  return `data/acceptance-${now.toISOString().replaceAll(":", "-")}.json`;
}

export function parseArgs(argv, now = new Date()) {
  const options = {
    baseUrl: DEFAULT_BASE_URL,
    durationMs: 60_000,
    intervalMs: 1_000,
    requestTimeoutMs: 3_000,
    output: defaultOutput(now),
    execution: "unknown",
    includeRuntimePreset: false,
    help: false,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === "--help") options.help = true;
    else if (flag === "--observe") { /* The only supported mode is read-only. */ }
    else if (flag === "--include-runtime-preset") options.includeRuntimePreset = true;
    else if (["--base-url", "--duration-seconds", "--interval-ms", "--output", "--execution"].includes(flag)) {
      const value = argv[++index];
      if (value === undefined || value.startsWith("--")) throw new Error(`${flag} requires a value`);
      if (flag === "--base-url") options.baseUrl = cleanOrigin(value);
      if (flag === "--duration-seconds") options.durationMs = integer(value, flag, 1, 3600) * 1000;
      if (flag === "--interval-ms") options.intervalMs = integer(value, flag, 100, 10000);
      if (flag === "--output") options.output = value;
      if (flag === "--execution") options.execution = value;
    } else throw new Error(`unknown option: ${flag}`);
  }
  options.baseUrl = cleanOrigin(options.baseUrl);
  if (!options.output) throw new Error("--output must not be empty");
  if (!["simulated", "physical", "unknown"].includes(options.execution)) {
    throw new Error("--execution must be simulated, physical, or unknown");
  }
  return options;
}

function scalar(value, fallback = null) {
  return typeof value === "string" || typeof value === "boolean" || Number.isFinite(value) ? value : fallback;
}

function id(value) {
  return typeof value === "string" && value.length <= 256 ? value : null;
}

function list(value, maximum = 10_000) {
  if (!Array.isArray(value)) return [];
  return value.slice(0, maximum);
}

function sanitize(name, value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid_shape");
  if (name === "status") return {
    protocol_version: scalar(value.protocol_version), session_id: id(value.session_id),
    bridge_connected: scalar(value.bridge_connected), generation: scalar(value.generation),
    speeches: list(value.speeches).map(item => ({ id: id(item?.id), generation: scalar(item?.generation), status: scalar(item?.status) })),
  };
  if (name === "agent") return {
    paused: scalar(value.paused), phase: scalar(value.phase), current_speech_id: id(value.current_speech_id),
    llm_configured: scalar(value.llm_configured), bridge_connected: scalar(value.bridge_connected),
    events: list(value.events).map(item => ({ event_id: id(item?.event?.id), status: scalar(item?.status), speech_id: id(item?.speech_id) })),
  };
  if (name === "live") return {
    platform: scalar(value.platform), configured: scalar(value.configured), phase: scalar(value.phase),
    accepted_events: scalar(value.accepted_events), duplicate_events: scalar(value.duplicate_events),
    rejected_events: scalar(value.rejected_events), reconnect_attempts: scalar(value.reconnect_attempts),
  };
  if (name === "training") return {
    enabled: scalar(value.enabled), busy: scalar(value.busy),
    jobs: list(value.jobs).map(item => ({ id: id(item?.id), status: scalar(item?.status), progress: scalar(item?.progress), clip_count: scalar(item?.clip_count) })),
    versions: list(value.versions).map(item => ({ id: id(item?.id), job_id: id(item?.job_id), auditioned: scalar(item?.auditioned), active: scalar(item?.active), available: scalar(item?.available) })),
  };
  if (name === "runtime_preset") {
    const measurement = value.measurement && typeof value.measurement === "object" ? {
      measured_at_ms: scalar(value.measurement.measured_at_ms), llm_ms: scalar(value.measurement.llm_ms),
      tts_ms: scalar(value.measurement.tts_ms), total_ms: scalar(value.measurement.total_ms),
      gpu_total_mib: scalar(value.measurement.gpu_total_mib), gpu_peak_used_mib: scalar(value.measurement.gpu_peak_used_mib),
      passed: scalar(value.measurement.passed),
    } : null;
    return { mode: scalar(value.mode), local_only: scalar(value.local_only), verified: scalar(value.verified), max_tokens: scalar(value.max_tokens), timeout_seconds: scalar(value.timeout_seconds), measurement };
  }
  throw new Error("unknown_endpoint");
}

async function boundedJson(response, signal) {
  const declared = Number(response.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > MAX_RESPONSE_BYTES) throw new Error("response_too_large");
  const reader = response.body?.getReader();
  if (!reader) throw new Error("invalid_json");
  const chunks = [];
  let size = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > MAX_RESPONSE_BYTES) {
      await reader.cancel();
      throw new Error("response_too_large");
    }
    chunks.push(value);
    if (signal.aborted) throw signal.reason;
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
  try { return JSON.parse(new TextDecoder().decode(bytes)); } catch { throw new Error("invalid_json"); }
}

async function query(baseUrl, name, endpoint, originNs, timeoutMs) {
  const started = process.hrtime.bigint();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), Math.min(timeoutMs, 3000));
  const sample = { endpoint, offset_ms: Number(started - originNs) / 1e6, query_ms: null, ok: false, status_code: null, error_summary: null, data: null };
  try {
    const response = await fetch(`${baseUrl}${endpoint}`, { method: "GET", signal: controller.signal, redirect: "error", headers: { accept: "application/json" } });
    sample.status_code = response.status;
    if (!response.ok) { sample.error_summary = `http_${response.status}`; await response.body?.cancel(); return sample; }
    sample.data = sanitize(name, await boundedJson(response, controller.signal));
    sample.ok = true;
    return sample;
  } catch (error) {
    sample.error_summary = controller.signal.aborted ? "request_timeout" : (["response_too_large", "invalid_json", "invalid_shape"].includes(error?.message) ? error.message : "request_failed");
    return sample;
  } finally {
    clearTimeout(timer);
    sample.query_ms = Number(process.hrtime.bigint() - started) / 1e6;
  }
}

function percentile(values, fraction) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return Math.round(sorted[Math.ceil(fraction * sorted.length) - 1] * 1000) / 1000;
}

function newTaskCounts() { return { completed: 0, failed: 0, cancelled: 0, unknown: 0 }; }

function summarize(samples) {
  const baseline = { speeches: new Set(), events: new Set(), training: new Set() };
  const observed = { speeches: new Map(), events: new Map(), training: new Map() };
  const initialized = new Set();
  const lastIds = { speeches: new Set(), events: new Set(), training: new Set() };
  let queuePeak = 0, interruptions = 0, restarts = 0, lowerBound = false;
  let previousSession, previousBridge, previousLivePhase;
  const latencies = [];
  for (const round of samples) {
    for (const sample of round.requests) {
      latencies.push(sample.query_ms);
      if (!sample.ok) continue;
      if (sample.endpoint === "/api/status") {
        const current = new Set(sample.data.speeches.map(task => task.id).filter(Boolean));
        queuePeak = Math.max(queuePeak, sample.data.speeches.filter(task => ACTIVE_SPEECH.has(task.status)).length);
        if (!initialized.has("speeches")) { current.forEach(value => baseline.speeches.add(value)); initialized.add("speeches"); }
        else for (const task of sample.data.speeches) if (!baseline.speeches.has(task.id)) observed.speeches.set(task.id, task.status);
        if ([...lastIds.speeches].some(value => !current.has(value))) lowerBound = true;
        lastIds.speeches = current;
        if (previousSession !== undefined && sample.data.session_id !== previousSession) { restarts += 1; lowerBound = true; }
        if (previousBridge === true && sample.data.bridge_connected === false) interruptions += 1;
        previousSession = sample.data.session_id; previousBridge = sample.data.bridge_connected;
      }
      if (sample.endpoint === "/api/agent") {
        const current = new Set(sample.data.events.map(event => event.event_id).filter(Boolean));
        if (!initialized.has("events")) { current.forEach(value => baseline.events.add(value)); initialized.add("events"); }
        else for (const event of sample.data.events) if (!baseline.events.has(event.event_id)) observed.events.set(event.event_id, event.status);
        if ([...lastIds.events].some(value => !current.has(value))) lowerBound = true;
        lastIds.events = current;
      }
      if (sample.endpoint === "/api/training") {
        const current = new Set(sample.data.jobs.map(job => job.id).filter(Boolean));
        if (!initialized.has("training")) { current.forEach(value => baseline.training.add(value)); initialized.add("training"); }
        else for (const job of sample.data.jobs) if (!baseline.training.has(job.id)) observed.training.set(job.id, job.status);
        if ([...lastIds.training].some(value => !current.has(value))) lowerBound = true;
        lastIds.training = current;
      }
      if (sample.endpoint === "/api/live") {
        if (["connected"].includes(previousLivePhase) && sample.data.phase !== "connected") interruptions += 1;
        previousLivePhase = sample.data.phase;
      }
    }
  }
  const count = map => { const result = newTaskCounts(); for (const status of map.values()) if (status in result) result[status] += 1; return result; };
  const eventStatuses = {};
  for (const status of observed.events.values()) eventStatuses[status] = (eventStatuses[status] ?? 0) + 1;
  return {
    new_tasks: { speeches: count(observed.speeches), training: count(observed.training) },
    unique_event_final_statuses: eventStatuses,
    queue_peak: queuePeak,
    api_query_ms: { p50: percentile(latencies, 0.50), p95: percentile(latencies, 0.95), label: "HTTP status query latency; not LLM or first-audio latency" },
    connection_interruptions: interruptions, service_restarts: restarts,
    counts_are_lower_bounds: lowerBound,
    lower_bound_reason: lowerBound ? "history_truncation_or_service_restart_observed" : null,
    resource_metrics: { process_memory_mib: null, cpu_percent: null, gpu_peak_used_mib: null, status: "not_measured" },
  };
}

function sleep(milliseconds) { return new Promise(resolve => setTimeout(resolve, milliseconds)); }

export async function collectAcceptance(options) {
  const baseUrl = cleanOrigin(options.baseUrl);
  const output = path.resolve(options.output);
  await mkdir(path.dirname(output), { recursive: true });
  const file = await open(output, "wx").catch(error => {
    if (error?.code === "EEXIST") throw new Error(`output already exists: ${output}`);
    throw error;
  });
  const startedAt = new Date();
  const originNs = process.hrtime.bigint();
  const endpoints = options.includeRuntimePreset ? [...ENDPOINTS, ["runtime_preset", "/api/runtime/preset"]] : ENDPOINTS;
  const samples = [];
  try {
    let round = 0;
    while (true) {
      const elapsed = Number(process.hrtime.bigint() - originNs) / 1e6;
      samples.push({ offset_ms: elapsed, requests: await Promise.all(endpoints.map(([name, endpoint]) => query(baseUrl, name, endpoint, originNs, options.requestTimeoutMs ?? 3000))) });
      round += 1;
      const after = Number(process.hrtime.bigint() - originNs) / 1e6;
      if (after >= options.durationMs) break;
      await sleep(Math.max(0, Math.min(options.durationMs - after, round * options.intervalMs - after)));
    }
    const endedAt = new Date();
    const report = {
      schema_version: 1, overall_acceptance: "manual_required", mode: "observe", execution: options.execution,
      target_origin: baseUrl, started_at: startedAt.toISOString(), ended_at: endedAt.toISOString(),
      observed_duration_ms: Number(process.hrtime.bigint() - originNs) / 1e6,
      requested_duration_ms: options.durationMs, interval_ms: options.intervalMs,
      request_timeout_ms: Math.min(options.requestTimeoutMs ?? 3000, 3000),
      endpoints: endpoints.map(([, endpoint]) => endpoint), samples,
      summary: summarize(samples),
      manual_evidence_required: ["physical_audio", "VTube_Studio_motion", "OBS_recording", "resource_measurement", "operator_result"],
    };
    await file.writeFile(`${JSON.stringify(report, null, 2)}\n`);
    return report;
  } finally {
    await file.close();
  }
}

async function main() {
  try {
    const options = parseArgs(process.argv.slice(2));
    if (options.help) { process.stdout.write(`${usage()}\n`); return; }
    const report = await collectAcceptance(options);
    process.stdout.write(`Acceptance observation saved to ${path.resolve(options.output)} (${Math.round(report.observed_duration_ms)} ms). Manual review is required.\n`);
  } catch (error) {
    process.stderr.write(`acceptance: ${error.message}\n`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) await main();
