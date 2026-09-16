import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer, request } from 'node:http';
import { createLauncherMiddleware } from './http.mjs';

async function fixture(t) {
  const calls = [];
  const manager = { token: 'test-token', async snapshot() { return { session_token: this.token }; },
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
