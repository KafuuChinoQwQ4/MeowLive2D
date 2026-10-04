import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync, verify } from 'node:crypto';
import { buildManifest } from './update-manifest.mjs';
test('manifest signs exact bytes and shares unchanged content blocks', () => {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const bytes = Buffer.concat([Buffer.alloc(1024 * 1024, 7), Buffer.from('tail')]);
  const result = buildManifest(bytes, 'v0.1.1-windows-preview.20261004', 'app-setup.exe', privateKey);
  const payload = Buffer.from(result.envelope.payload, 'base64');
  assert.equal(verify(null, payload, publicKey, Buffer.from(result.envelope.signature, 'base64')), true);
  assert.equal(result.manifest.sha256, createHash('sha256').update(bytes).digest('hex'));
  assert.equal(result.manifest.chunks.length, 2);
  assert.equal(result.chunks.get(result.manifest.chunks[0].sha256).length, 1024 * 1024);
  assert.equal(verify(null, Buffer.concat([payload, Buffer.from('x')]), publicKey, Buffer.from(result.envelope.signature, 'base64')), false);
});
test('successive releases reuse the unchanged block and reject empty installers', () => {
  const { privateKey } = generateKeyPairSync('ed25519');
  const unchanged = Buffer.alloc(1024 * 1024, 9);
  const first = buildManifest(Buffer.concat([unchanged, Buffer.from('old')]), 'v0.1.1-windows-preview.20261003', 'app-setup.exe', privateKey);
  const next = buildManifest(Buffer.concat([unchanged, Buffer.from('new')]), 'v0.1.1-windows-preview.20261004', 'app-setup.exe', privateKey);
  assert.equal(first.manifest.chunks[0].sha256, next.manifest.chunks[0].sha256);
  assert.notEqual(first.manifest.chunks[1].sha256, next.manifest.chunks[1].sha256);
  assert.throws(() => buildManifest(Buffer.alloc(0), 'v0.1.1', 'app-setup.exe', privateKey));
  assert.throws(() => buildManifest(Buffer.from('x'), 'v0.1.1', '../app-setup.exe', privateKey));
});
test('content boundaries recover after an inserted prefix', () => {
  const { privateKey } = generateKeyPairSync('ed25519');
  const original = Buffer.alloc(6 * 1024 * 1024);
  let state = 1234567;
  for (let i = 0; i < original.length; i++) { state ^= state << 13; state ^= state >>> 17; state ^= state << 5; original[i] = state & 255; }
  const before = buildManifest(original, 'v0.1.1', 'app-setup.exe', privateKey);
  const after = buildManifest(Buffer.concat([Buffer.from('inserted-header'), original]), 'v0.1.2', 'app-setup.exe', privateKey);
  const reused = after.manifest.chunks.filter((chunk) => before.chunks.has(chunk.sha256)).reduce((sum, chunk) => sum + chunk.size, 0);
  assert.ok(reused > original.length / 2, `Expected substantial unchanged data reuse, got ${reused}`);
});
