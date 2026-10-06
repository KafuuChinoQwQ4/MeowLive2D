import { copyFile, mkdir, readdir, readFile, rename, unlink, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const chunkPattern = /^chunk-[a-f0-9]{64}\.bin$/;
function platform(name) {
  const system = /\.(exe|msi)$/i.test(name) ? 'Windows'
    : /\.(dmg|pkg)$/i.test(name) ? 'macOS'
      : /\.(AppImage|deb|rpm)$/i.test(name) ? 'Linux' : null;
  if (!system) return null;
  const arch = /(?:^|[_\-.])(aarch64|arm64)(?:[_\-.]|$)/i.test(name) ? 'ARM64'
    : /(?:^|[_\-.])(x64|x86_64|amd64)(?:[_\-.]|$)/i.test(name) ? 'x64' : '';
  return `${system} ${arch}`.trim();
}

export async function prepareRelease(directory, updateDirectory, tag, body, repository) {
  if (!/^v?\d+\.\d+\.\d+(?:-windows-preview\.\d{8})?$/.test(tag)) throw new Error('Invalid release tag');
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error('Invalid repository');
  const names = (await readdir(directory, { withFileTypes: true })).filter((entry) => entry.isFile()).map((entry) => entry.name).sort();
  const chunks = names.filter((name) => chunkPattern.test(name));
  const manifests = names.filter((name) => name === 'meowlive-update.json' || /^meowlive-update-linux-(appimage|deb)\.json$/.test(name));
  const chunkGroups = new Map();
  for (const manifestName of manifests) {
    const envelope = JSON.parse(await readFile(resolve(directory, manifestName), 'utf8'));
    const payload = JSON.parse(Buffer.from(envelope.payload, 'base64').toString('utf8'));
    const target = payload.target ?? 'windows-x64';
    const group = target.startsWith('linux-') ? 'linux-x86_64' : 'windows-x86_64';
    if (!chunkGroups.has(group)) chunkGroups.set(group, new Set());
    for (const chunk of payload.chunks ?? []) chunkGroups.get(group).add(`chunk-${chunk.sha256}.bin`);
  }
  if (chunks.length && !chunkGroups.size) throw new Error('Chunks require a signed update manifest');
  const hasUpdates = chunkGroups.size > 0;
  await mkdir(updateDirectory, { recursive: true });
  const chunkLocations = new Map();
  for (const [group, expected] of chunkGroups) {
    const output = group === 'windows-x86_64' ? updateDirectory : resolve(updateDirectory, group);
    await mkdir(output, { recursive: true });
    for (const name of expected) {
      if (!chunks.includes(name)) throw new Error(`Signed update chunk is missing: ${name}`);
      const source = resolve(directory, name);
      const destination = resolve(output, name);
      const previous = chunkLocations.get(name);
      if (previous) await copyFile(previous, destination);
      else {
        await rename(source, destination);
        chunkLocations.set(name, destination);
      }
    }
  }
  for (const name of chunks) {
    try { await unlink(resolve(directory, name)); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
  const downloads = names.filter((name) => platform(name));
  if (!downloads.length) throw new Error('No platform installers found');
  const updateTag = `${tag}-updates-windows-x86_64`;
  const linuxUpdateTag = `${tag}-updates-linux-x86_64`;
  const rows = downloads.map((name) => `| ${platform(name)} | [下载安装包](https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(name)}) |`);
  const notes = ['## 下载', '', '| 平台 | 安装包 |', '| --- | --- |', ...rows, '',
    '普通用户只需下载对应平台的安装包。校验和及更新清单供校验或 App 自动更新使用。', '',
    ...(hasUpdates ? [`增量资源已单独存放，由 App 自动下载：[Windows 更新资源](https://github.com/${repository}/releases/tag/${updateTag})${chunkGroups.has('linux-x86_64') ? `、[Linux 更新资源](https://github.com/${repository}/releases/tag/${linuxUpdateTag})` : ''}。`, ''] : []),
    '---', '', body].join('\n');
  return { notes, hasUpdates, updateTag, linuxUpdateTag: chunkGroups.has('linux-x86_64') ? linuxUpdateTag : null, updateGroups: [...chunkGroups.keys()] };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [directory, updateDirectory, sourceNotes, outputNotes] = process.argv.slice(2);
  const result = await prepareRelease(directory, updateDirectory, process.env.RELEASE_TAG, await readFile(sourceNotes, 'utf8'), process.env.GITHUB_REPOSITORY);
  await writeFile(outputNotes, result.notes);
}
