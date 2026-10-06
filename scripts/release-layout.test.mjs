import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readdir, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { prepareRelease } from './release-layout.mjs';

test('public downloads stay separate from chunks and list available platforms', async () => {
  const root = await mkdtemp(join(tmpdir(), 'meowlive-release-'));
  try {
    const files = ['MeowLive2D_0.1.2_x64-setup.exe', 'MeowLive2D_0.1.2_aarch64.dmg', 'MeowLive2D_0.1.2_amd64.AppImage', 'meowlive-desktop_0.1.2_amd64.deb', 'SHA256SUMS.txt', 'build-info.json', 'meowlive-update.json', 'meowlive-update-linux-appimage.json', 'meowlive-update-linux-deb.json', `chunk-${'a'.repeat(64)}.bin`];
    for (const name of files) await writeFile(join(root, name), name);
    const signedPayload = (target) => JSON.stringify({ payload: Buffer.from(JSON.stringify({ target, chunks: [{ sha256: 'a'.repeat(64) }] })).toString('base64'), signature: 'test-signature' });
    await writeFile(join(root, 'meowlive-update.json'), signedPayload('windows-x64'));
    await writeFile(join(root, 'meowlive-update-linux-appimage.json'), signedPayload('linux-appimage-x86_64'));
    await writeFile(join(root, 'meowlive-update-linux-deb.json'), signedPayload('linux-deb-x86_64'));
    const result = await prepareRelease(root, join(root, 'updates'), '0.1.3', 'Release details', 'KafuuChinoQwQ4/MeowLive2D');
    assert.deepEqual((await readdir(join(root, 'updates'))), [`chunk-${'a'.repeat(64)}.bin`, 'linux-x86_64']);
    assert.deepEqual(await readdir(join(root, 'updates', 'linux-x86_64')), [`chunk-${'a'.repeat(64)}.bin`]);
    assert.equal(await readFile(join(root, 'updates', files.at(-1)), 'utf8'), files.at(-1));
    assert.ok(!(await readdir(root)).includes(files.at(-1)));
    assert.ok((await readdir(root)).includes('meowlive-update.json'));
    assert.match(result.notes, /Windows.*x64/);
    assert.match(result.notes, /macOS.*ARM64/);
    assert.match(result.notes, /Linux.*x64/);
    assert.match(result.notes, /releases\/download\/0\.1\.3\/MeowLive2D_0\.1\.2_x64-setup\.exe/);
    assert.ok(result.notes.indexOf('下载') < result.notes.indexOf('Release details'));
    assert.ok(!result.notes.includes('chunk-'));
    assert.equal(result.updateTag, '0.1.3-updates-windows-x86_64');
    assert.equal(result.linuxUpdateTag, '0.1.3-updates-linux-x86_64');
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('manual-only releases do not advertise nonexistent platforms or updates', async () => {
  const root = await mkdtemp(join(tmpdir(), 'meowlive-release-'));
  try {
    await writeFile(join(root, 'app_x64-setup.exe'), 'installer');
    const result = await prepareRelease(root, join(root, 'updates'), '0.1.3', 'Details', 'owner/repo');
    assert.equal(result.hasUpdates, false);
    assert.ok(!result.notes.includes('macOS'));
    assert.ok(!result.notes.includes('Linux'));
    assert.ok(!result.notes.includes('updates-windows'));
  } finally { await rm(root, { recursive: true, force: true }); }
});
