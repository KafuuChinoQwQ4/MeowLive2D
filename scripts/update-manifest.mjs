import { createHash, createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { readFile, mkdir, writeFile, readdir, stat } from 'node:fs/promises';
import { resolve, basename } from 'node:path';
import { pathToFileURL } from 'node:url';
const MAX_INSTALLER = 2 * 1024 * 1024 * 1024 - 1;
const gear = Array.from({ length: 256 }, (_, index) => createHash('sha256').update(`MeowLive2D-chunk-${index}`).digest().readUInt32LE());
// Content-defined boundaries recover after shifted executable/resource offsets.
export function* splitChunks(bytes) {
  let start = 0;
  let rolling = 0;
  for (let index = 0; index < bytes.length; index++) {
    rolling = ((rolling << 1) + gear[bytes[index]]) >>> 0;
    const size = index - start + 1;
    if (size >= 1048576 || (size >= 262144 && (rolling & 0x7ffff) === 0)) {
      yield bytes.subarray(start, index + 1);
      start = index + 1;
      rolling = 0;
    }
  }
  if (start < bytes.length) yield bytes.subarray(start);
}
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
export function targetForInstaller(installer) {
  if (/\.AppImage$/i.test(installer)) return 'linux-appimage-x86_64';
  if (/\.deb$/i.test(installer)) return 'linux-deb-x86_64';
  if (/-setup\.exe$/i.test(installer)) return 'windows-x64';
  throw new Error('Unsupported installer type');
}
export function manifestNameForTarget(target) {
  const names = { 'windows-x64': 'meowlive-update.json', 'linux-appimage-x86_64': 'meowlive-update-linux-appimage.json', 'linux-deb-x86_64': 'meowlive-update-linux-deb.json' };
  if (!names[target]) throw new Error('Unsupported update target');
  return names[target];
}
export function buildManifest(bytes, tag, installer, privateKey, target = targetForInstaller(installer)) {
  if (bytes.length === 0 || bytes.length > MAX_INSTALLER) throw new Error('Invalid installer size');
  if (!/^v?\d+\.\d+\.\d+(?:-windows-preview\.\d{8})?$/.test(tag)) throw new Error('Invalid release tag');
  if (!/^[\w. -]+(?:-setup\.exe|\.AppImage|\.deb)$/.test(installer) || target !== targetForInstaller(installer)) throw new Error('Invalid installer name or target');
  const chunks = new Map();
  const blocks = [];
  for (const block of splitChunks(bytes)) {
    const sha256 = hash(block);
    chunks.set(sha256, block);
    blocks.push({ sha256, size: block.length });
  }
  if (chunks.size > 990) throw new Error('Too many update chunks for GitHub Releases; installer must be smaller');
  const manifest = { schema: 1, tag, target, installer, size: bytes.length, sha256: hash(bytes), chunks: blocks };
  const payload = Buffer.from(JSON.stringify(manifest));
  return { manifest, chunks, envelope: { payload: payload.toString('base64'), signature: sign(null, payload, privateKey).toString('base64') } };
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const dir = resolve(process.argv[2] || 'release-assets');
  const key = process.env.MEOWLIVE_UPDATE_PRIVATE_KEY;
  const publicKey = process.env.MEOWLIVE_UPDATE_PUBLIC_KEY;
  if (!key || !publicKey) throw new Error('Update signing configuration is required');
  const privateKey = createPrivateKey(key);
  if (privateKey.asymmetricKeyType !== 'ed25519') throw new Error('Update signing key must use Ed25519');
  const actual = createPublicKey(privateKey).export({ type: 'spki', format: 'der' }).subarray(-32).toString('base64');
  if (actual !== publicKey) throw new Error('Update signing public key does not match');
  const installers = (await readdir(dir)).filter((name) => /(?:-setup\.exe|\.AppImage|\.deb)$/i.test(name));
  if (!installers.length) throw new Error('No supported installers found');
  await mkdir(dir, { recursive: true });
  for (const installer of installers) {
    if ((await stat(resolve(dir, installer))).size > MAX_INSTALLER) throw new Error('Installer exceeds GitHub asset size limit');
    const result = buildManifest(await readFile(resolve(dir, installer)), process.env.RELEASE_TAG, basename(installer), privateKey);
    for (const [digest, bytes] of result.chunks) await writeFile(resolve(dir, `chunk-${digest}.bin`), bytes);
    await writeFile(resolve(dir, manifestNameForTarget(result.manifest.target)), JSON.stringify(result.envelope));
  }
}
