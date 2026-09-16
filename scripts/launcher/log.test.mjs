import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { captureOutput } from './log.mjs';

test('redacts secrets split across chunks and keeps actionable CUDA diagnostics', async t => {
  const root = await mkdtemp(join(tmpdir(), 'meow-log-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const path = join(root, 'service.log');
  const child = spawn(process.execPath, ['-e', `process.stdout.write('sk-test-'); setTimeout(() => console.log('secret custom-credential\\nCUDA out of memory'), 10)`], { stdio: ['ignore', 'pipe', 'pipe'] });
  let diagnostic;
  captureOutput(child, { logPath: path, env: { TEST_API_KEY: 'custom-credential' } }, value => { diagnostic = value; });
  await new Promise(resolve => child.once('close', resolve));
  const data = await readFile(path, 'utf8');
  assert.ok(!data.includes('sk-test-secret') && !data.includes('custom-credential'));
  assert.match(data, /REDACTED/);
  assert.match(diagnostic, /显存不足/);
});
