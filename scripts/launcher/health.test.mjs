import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { inspectEndpoint } from './health.mjs';
import { inspectBridge } from './windows-client.mjs';

test('authenticated server remains discoverable without exposing speech or administrator credentials', async t => {
  const server = createServer((req, res) => {
    assert.equal(req.headers.authorization, undefined);
    if (req.url !== '/api/health') { res.writeHead(401); res.end('{}'); return; }
    res.end(JSON.stringify({ service: 'meowlive', protocol_version: 3, bridge_connected: true }));
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => { server.closeAllConnections(); server.close(); });
  const url = `http://127.0.0.1:${server.address().port}`;
  assert.deepEqual(await inspectEndpoint({ id: 'server', url }), { healthy: true, occupied: true });
  assert.deepEqual(await inspectBridge(url), { ready: true, connected: true });
});
