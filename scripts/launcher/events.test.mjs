import test from 'node:test';
import assert from 'node:assert/strict';
import { appendFile, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import { LauncherEventStore } from './events.mjs';

test('lists up to 1000 persisted launcher events', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-events-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const path = join(root, 'events.jsonl');
  const store = new LauncherEventStore(path);
  await store.record('info', 'service_started', 'server');
  const template = (await store.list()).entries[0];
  await writeFile(path, Array.from({ length: 1000 }, () => JSON.stringify({ ...template, id: randomUUID() })).join('\n') + '\n');
  const restored = new LauncherEventStore(path);
  assert.equal((await restored.list({ limit: 1000 })).entries.length, 1000);
  assert.equal((await restored.list({ limit: 1001 })).entries.length, 1000);
});

test('launcher events are structured, bounded, and redact unsafe text', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-events-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const store = new LauncherEventStore(join(root, 'events.jsonl'), { maxEntries: 2 });
  await store.record('info', 'service_started', 'server', '主服务启动');
  await store.record('error', 'service_failed', 'server', 'token=secret');
  await store.record('warn', 'service_exited', 'tts', '第三方原始输出');
  const result = await store.list({ limit: 10 });
  assert.equal(result.entries.length, 2);
  assert.equal(result.entries[0].code, 'service_exited');
  assert.equal(JSON.stringify(result).includes('secret'), false);
  assert.equal(JSON.stringify(result).includes('第三方'), false);
  assert.equal(result.entries.every(entry => Object.keys(entry).sort().join(',') === 'category,code,id,level,source,summary,timestamp'), true);
  assert.ok((await readFile(join(root, 'events.jsonl'), 'utf8')).length < 4096);
  const restored = new LauncherEventStore(join(root, 'events.jsonl'));
  assert.deepEqual((await restored.list({ level: 'error', category: 'server' })).entries, [result.entries[1]]);
  await writeFile(join(root, 'events.jsonl'), JSON.stringify({ ...result.entries[1], summary: 'PRIVATE', raw: 'PRIVATE' }));
  const sanitized = await restored.list();
  assert.equal(JSON.stringify(sanitized).includes('PRIVATE'), false);
});

test('keyword filtering trims and searches source and category without case sensitivity', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-events-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const store = new LauncherEventStore(join(root, 'events.jsonl'));
  await store.record('info', 'service_started', 'tts');
  for (const query of [' LAUNCHER ', ' TTS ', ' SERVICE_STARTED ']) assert.equal((await store.list({ query })).entries.length, 1);
});

test('a partial persisted tail cannot swallow the first event after restart', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-events-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const path = join(root, 'events.jsonl');
  await new LauncherEventStore(path).record('info', 'service_started', 'server');
  await appendFile(path, '{"partial":');
  await new LauncherEventStore(path).record('info', 'service_ready', 'server');
  assert.deepEqual((await new LauncherEventStore(path).list()).entries.map(entry => entry.code), ['service_ready', 'service_started']);
});

test('pending writes are bounded and overflow remains visible in memory and status', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-events-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const store = new LauncherEventStore(join(root, 'events.jsonl'), { maxEntries: 2 });
  let release;
  store.queue = new Promise(resolve => { release = resolve; });
  const writes = Array.from({ length: 10 }, () => store.record('info', 'service_started', 'server'));
  assert.equal(store.pendingWrites, 2);
  assert.equal(store.memory.length, 2);
  release(); await Promise.all(writes);
  assert.equal(store.pendingWrites, 0);
  const result = await store.list();
  assert.equal(result.entries.length, 2);
  assert.equal(result.truncated, true);
  assert.equal(result.storage_available, false);
});

test('unwritable persistence keeps a bounded in-memory event view', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-events-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await writeFile(join(root, 'blocked'), 'file');
  const store = new LauncherEventStore(join(root, 'blocked/events.jsonl'), { maxEntries: 2 });
  for (let i = 0; i < 3; i++) await store.record('error', 'service_failed', 'server');
  const result = await store.list();
  assert.equal(result.entries.length, 2);
  assert.equal(result.storage_available, false);
});
