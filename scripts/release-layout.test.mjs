import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readdir, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { prepareRelease } from './release-layout.mjs';

test('public downloads stay separate from chunks and list available platforms', async () => {
  const root = await mkdtemp(join(tmpdir(), 'meowlive-release-'));
  try {
    const files = ['MeowLive2D_0.1.2_x64-setup.exe', 'MeowLive2D_0.1.2_aarch64.dmg', 'MeowLive2D_0.1.2_amd64.AppImage', 'SHA256SUMS.txt', 'build-info.json', 'meowlive-update.json', `chunk-${'a'.repeat(64)}.bin`];
    for (const name of files) await writeFile(join(root, name), name);
    const result = await prepareRelease(root, join(root, 'updates'), 'v0.1.2', 'Release details', 'KafuuChinoQwQ4/MeowLive2D');
    assert.deepEqual((await readdir(join(root, 'updates'))), [`chunk-${'a'.repeat(64)}.bin`]);
    assert.equal(await readFile(join(root, 'updates', files.at(-1)), 'utf8'), files.at(-1));
    assert.ok(!(await readdir(root)).includes(files.at(-1)));
    assert.ok((await readdir(root)).includes('meowlive-update.json'));
    assert.match(result.notes, /Windows.*x64/);
    assert.match(result.notes, /macOS.*ARM64/);
    assert.match(result.notes, /Linux.*x64/);
    assert.match(result.notes, /releases\/download\/v0\.1\.2\/MeowLive2D_0\.1\.2_x64-setup\.exe/);
    assert.ok(result.notes.indexOf('下载') < result.notes.indexOf('Release details'));
    assert.ok(!result.notes.includes('chunk-'));
    assert.equal(result.updateTag, 'v0.1.2-updates-windows-x86_64');
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('manual-only releases do not advertise nonexistent platforms or updates', async () => {
  const root = await mkdtemp(join(tmpdir(), 'meowlive-release-'));
  try {
    await writeFile(join(root, 'app_x64-setup.exe'), 'installer');
    const result = await prepareRelease(root, join(root, 'updates'), 'v0.1.2', 'Details', 'owner/repo');
    assert.equal(result.hasUpdates, false);
    assert.ok(!result.notes.includes('macOS'));
    assert.ok(!result.notes.includes('Linux'));
    assert.ok(!result.notes.includes('updates-windows'));
  } finally { await rm(root, { recursive: true, force: true }); }
});
