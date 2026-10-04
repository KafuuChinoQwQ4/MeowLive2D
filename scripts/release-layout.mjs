import { mkdir, readdir, readFile, rename, writeFile } from 'node:fs/promises';
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
  if (!/^v\d+\.\d+\.\d+(?:-windows-preview\.\d{8})?$/.test(tag)) throw new Error('Invalid release tag');
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error('Invalid repository');
  const names = (await readdir(directory, { withFileTypes: true })).filter((entry) => entry.isFile()).map((entry) => entry.name).sort();
  const chunks = names.filter((name) => chunkPattern.test(name));
  const hasUpdates = chunks.length > 0;
  if (hasUpdates && !names.includes('meowlive-update.json')) throw new Error('Chunks require a signed update manifest');
  await mkdir(updateDirectory, { recursive: true });
  for (const name of chunks) await rename(resolve(directory, name), resolve(updateDirectory, name));
  const downloads = names.filter((name) => platform(name));
  if (!downloads.length) throw new Error('No platform installers found');
  const updateTag = `${tag}-updates-windows-x86_64`;
  const rows = downloads.map((name) => `| ${platform(name)} | [下载安装包](https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(name)}) |`);
  const notes = ['## 下载', '', '| 平台 | 安装包 |', '| --- | --- |', ...rows, '',
    '普通用户只需下载对应平台的安装包。校验和及更新清单供校验或 App 自动更新使用。', '',
    ...(hasUpdates ? [`增量资源已单独存放，由 App 自动下载：[更新资源](https://github.com/${repository}/releases/tag/${updateTag})。`, ''] : []),
    '---', '', body].join('\n');
  return { notes, hasUpdates, updateTag };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [directory, updateDirectory, sourceNotes, outputNotes] = process.argv.slice(2);
  const result = await prepareRelease(directory, updateDirectory, process.env.RELEASE_TAG, await readFile(sourceNotes, 'utf8'), process.env.GITHUB_REPOSITORY);
  await writeFile(outputNotes, result.notes);
}
