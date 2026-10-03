import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer, request } from 'node:http';
import { createLauncherMiddleware } from './http.mjs';

async function fixture(t, events) {
  const calls = [];
  const manager = { token: 'test-token', events, async snapshot() { return { session_token: this.token }; },
    async setEnabled(...args) { calls.push(args); } };
  const models = { async snapshot() { return { catalog: [] }; }, async action(...args) { calls.push(args); } };
  const server = createServer((req, res) => middleware(req, res, () => { res.writeHead(404).end(); }));
  let middleware;
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  middleware = createLauncherMiddleware(manager, port, models);
  t.after(() => new Promise(resolve => server.close(resolve)));
  const base = `http://127.0.0.1:${port}`;
  const headers = { Origin: base, 'Content-Type': 'application/json', 'X-MeowLive-Launcher-Token': 'test-token' };
  return { base, headers, calls, port };
}

test('same-origin authenticated switch requests reach only the named service', async t => {
  const { base, headers, calls } = await fixture(t);
  const response = await fetch(`${base}/api/launcher/services/tts`, { method: 'POST', headers, body: '{"enabled":true}' });
  assert.equal(response.status, 200);
  assert.deepEqual(calls, [['tts', true]]);
  const windows = await fetch(`${base}/api/launcher/services/windows`, { method: 'POST', headers, body: '{"enabled":true}' });
  assert.equal(windows.status, 200);
  assert.deepEqual(calls.at(-1), ['windows', true]);
});

test('same-origin launcher logs expose only bounded structured events without a token', async t => {
  const { base } = await fixture(t);
  const response = await fetch(`${base}/api/launcher/logs?limit=10`, { headers: { Origin: base } });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { entries: [], storage_available: false, truncated: false });
});

test('cross-origin and missing-token requests cannot operate services', async t => {
  const { base, headers, calls } = await fixture(t);
  for (const altered of [{ ...headers, Origin: 'https://untrusted.example' }, { ...headers, 'X-MeowLive-Launcher-Token': '' }]) {
    assert.equal((await fetch(`${base}/api/launcher/services/server`, { method: 'POST', headers: altered, body: '{"enabled":true}' })).status, 403);
  }
  assert.deepEqual(calls, []);
});

test('unknown command fields, invalid bools, wrong media type and oversize bodies are rejected', async t => {
  const { base, headers, calls } = await fixture(t);
  for (const body of ['{"enabled":"yes"}', '{"enabled":true,"command":"anything"}', '{}', 'null']) {
    assert.equal((await fetch(`${base}/api/launcher/services/server`, { method: 'POST', headers, body })).status, 400);
  }
  assert.equal((await fetch(`${base}/api/launcher/services/server`, { method: 'POST', headers: { ...headers, 'Content-Type': 'text/plain' }, body: '{}' })).status, 415);
  assert.equal((await fetch(`${base}/api/launcher/services/server`, { method: 'POST', headers, body: 'a'.repeat(2048) })).status, 413);
  assert.deepEqual(calls, []);
});

test('a rebound Host cannot read a launcher token', async t => {
  const { port } = await fixture(t);
  const status = await new Promise((resolve, reject) => {
    const req = request({ host: '127.0.0.1', port, path: '/api/launcher/status', headers: { Host: `untrusted.example:${port}` } }, res => { res.resume(); resolve(res.statusCode); });
    req.on('error', reject); req.end();
  });
  assert.equal(status, 403);
});

test('launcher logs are same-origin readable without exposing raw output', async t => {
  const { base } = await fixture(t);
  const response = await fetch(`${base}/api/launcher/logs?limit=100`);
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { entries: [], storage_available: false, truncated: false });
  assert.equal((await fetch(`${base}/api/launcher/logs?limit=0`)).status, 400);
  assert.equal((await fetch(`${base}/api/launcher/logs?limit=1000`)).status, 200);
  assert.equal((await fetch(`${base}/api/launcher/logs?limit=1001`)).status, 400);
  assert.equal((await fetch(`${base}/api/launcher/logs?message=secret`)).status, 400);
});

test('launcher log filters reach the shared store and cross-origin reads are denied', async t => {
  let query;
  const events = { async list(value) { query = value; return { entries: [], storage_available: true, truncated: false }; } };
  const { base } = await fixture(t, events);
  assert.equal((await fetch(`${base}/api/launcher/logs?source=server&category=tts&level=error&query=failed&limit=20`)).status, 200);
  assert.deepEqual(query, { source: 'server', category: 'tts', level: 'error', query: 'failed', limit: 20 });
  assert.equal((await fetch(`${base}/api/launcher/logs`, { headers: { Origin: 'http://untrusted.example' } })).status, 403);
});

test('model actions require the launcher token and accept only fixed identifiers', async t => {
  const { base, headers, calls } = await fixture(t);
  const send = (action, body, requestHeaders = headers) => fetch(`${base}/api/launcher/models/${action}`, {
    method: 'POST', headers: requestHeaders, body: JSON.stringify(body),
  });
  assert.equal((await send('download', { id: 'gpt-sovits-v2' }, { ...headers, 'X-MeowLive-Launcher-Token': '' })).status, 403);
  assert.equal((await send('download', { id: '../escape' })).status, 400);
  assert.equal((await send('download', { id: 'gpt-sovits-v2', url: 'https://example.com' })).status, 400);
  assert.equal((await send('scan', {})).status, 200);
  assert.equal((await send('download', { id: 'gpt-sovits-v2' })).status, 200);
  assert.deepEqual(calls, [['scan', undefined], ['download', 'gpt-sovits-v2']]);
});
