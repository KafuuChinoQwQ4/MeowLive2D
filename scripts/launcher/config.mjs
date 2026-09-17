import { access, copyFile, mkdir, readFile, chmod } from 'node:fs/promises';
import { constants } from 'node:fs';
import { resolve, join } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { detectEnvironment } from './model-environment.mjs';
import { homedir } from 'node:os';
import { displayPath, resolveConfiguredPath } from './paths.mjs';

const execute = promisify(execFile);
const defaults = { serverConfig: 'config/server.local.toml', ttsPython: 'data/environments/gpt-sovits/bin/python',
  ttsEngineRoot: 'data/engines/GPT-SoVITS', ttsDevice: 'cuda', ttsMemoryMode: 'low', llmKeyFile: 'config/local/llm-api-key.txt' };

async function exists(path, mode = constants.R_OK) {
  try { await access(path, mode); return true; } catch { return false; }
}

async function initialize(root, destination, template) {
  if (await exists(destination)) return;
  await mkdir(resolve(destination, '..'), { recursive: true, mode: 0o700 });
  try { await copyFile(join(root, template), destination, constants.COPYFILE_EXCL); await chmod(destination, 0o600); }
  catch (error) { if (error.code !== 'EEXIST') throw error; }
}

async function readServerConfig(path) {
  // Keep TOML parsing in Python's standard library; no cloud key or full config is printed.
  const script = `import json,sys,tomllib
from pathlib import Path
with open(sys.argv[1], 'rb') as f: data=tomllib.load(f)
profile=Path(sys.argv[1]).parent/'local'/(Path(sys.argv[1]).stem+'-llm.json')
llm=data.get('llm',{})
key_saved=False
if profile.exists():
 if profile.is_symlink() or profile.stat().st_size>16384: raise ValueError('invalid profile')
 saved=json.loads(profile.read_text())
 if saved.get('schema')!=1: raise ValueError('invalid profile')
 llm=saved['config']
 key_saved=bool(saved.get('api_key'))
print(json.dumps({
 'listen':data.get('server',{}).get('listen_address','127.0.0.1:19600'),
 'tts':data.get('speech',{}).get('base_url','http://127.0.0.1:9880'),
 'llm':{**{k:llm.get(k,'') for k in ['base_url','model','api_key_env']},'key_saved':key_saved}
}))`;
  const { stdout } = await execute('python3', ['-c', script, path], { timeout: 5000, maxBuffer: 65536 });
  return JSON.parse(stdout);
}

function localUrl(value) {
  const url = new URL(value);
  if (url.protocol !== 'http:' || !['127.0.0.1', 'localhost'].includes(url.hostname)
      || url.username || url.password || url.search || url.hash || url.pathname !== '/'
      || !url.port || Number(url.port) < 1024) throw new Error('non-local URL');
  url.hostname = '127.0.0.1';
  return url.origin;
}

export async function loadConfiguration(root, { env = process.env } = {}) {
  const configurationPath = join(root, 'config/local/launcher.json');
  const home = env.HOME || homedir();
  const resolvePath = value => resolveConfiguredPath(root, value, home);
  const publicPath = value => displayPath(value, root, home);
  let settings = { ...defaults }, commonIssue = null;
  try {
    await initialize(root, configurationPath, 'config/launcher.example.json');
    const data = JSON.parse(await readFile(configurationPath, 'utf8'));
    if (!data || Array.isArray(data) || typeof data !== 'object' || Object.keys(data).some(key => !(key in defaults))
        || Object.values(data).some(value => typeof value !== 'string' || !value.trim() || value.length > 4096)) throw new Error('invalid settings');
    settings = { ...defaults, ...data };
    if (!['cuda', 'cpu'].includes(settings.ttsDevice)) throw new Error('invalid device');
    if (!['low', 'standard'].includes(settings.ttsMemoryMode)) throw new Error('invalid memory mode');
  } catch { commonIssue = '启动配置 JSON 无效或无法读取。请检查 config/local/launcher.json，然后重启控制面板。'; }
  const environment = detectEnvironment(undefined, env);
  if (!environment.ready) commonIssue = environment.message;
  const serverConfig = resolvePath(settings.serverConfig);
  const python = resolvePath(settings.ttsPython), engine = resolvePath(settings.ttsEngineRoot);
  let serverUrl = 'http://127.0.0.1:19600', ttsUrl = 'http://127.0.0.1:9880';
  let serverIssue = commonIssue, ttsIssue = commonIssue, metadata;
  if (!commonIssue) {
    try {
      if (settings.serverConfig === defaults.serverConfig) await initialize(root, serverConfig, 'config/server.example.toml');
      metadata = await readServerConfig(serverConfig);
    } catch { serverIssue = ttsIssue = '无法读取主服务 TOML 配置。请确认文件存在、格式正确，且系统安装 Python 3.11 或更新版本，然后重启面板。'; }
  }
  if (metadata) {
    try { serverUrl = localUrl(`http://${metadata.listen}`); }
    catch { serverIssue = '网页启动器要求主服务监听 127.0.0.1 的独立端口，请修改主服务配置后重启面板。'; }
    try { ttsUrl = localUrl(metadata.tts); }
    catch { ttsIssue = '网页启动器要求 speech.base_url 使用 http://127.0.0.1:端口，请修改主服务配置后重启面板。'; }
    if ([serverUrl, ttsUrl].some(url => new URL(url).port === '1420') || serverUrl === ttsUrl) {
      serverIssue = ttsIssue = '网页、主服务和 TTS 必须使用不同端口（默认 1420、19600、9880）。';
    }
  }
  if (!ttsIssue && !await exists(python, constants.X_OK)) ttsIssue = '未找到可执行的 TTS Python。请在 config/local/launcher.json 设置 ttsPython 后重启面板。';
  if (!ttsIssue && !await exists(join(engine, 'api_v2.py'))) ttsIssue = '未找到 GPT-SoVITS 引擎。请在 config/local/launcher.json 设置 ttsEngineRoot 后重启面板。';
  const serverEnv = { ...env }, keyName = metadata?.llm?.api_key_env;
  if (typeof keyName === 'string' && /^[A-Za-z_][A-Za-z0-9_]*$/.test(keyName) && !serverEnv[keyName]) {
    try {
      const text = await readFile(resolvePath(settings.llmKeyFile), 'utf8');
      const key = text.split(/\r?\n/).map(line => line.trim()).find(line => /^sk-[A-Za-z0-9_-]+$/.test(line));
      if (key) serverEnv[keyName] = key;
    } catch { /* Manual speech remains available without a cloud key. */ }
  }
  const llmConfigured = Boolean(metadata?.llm?.base_url && metadata?.llm?.model && (metadata.llm.key_saved || !keyName || serverEnv[keyName]));
  const ttsEnv = Object.fromEntries(Object.entries(env).filter(([name]) => name !== keyName && !/KEY|TOKEN|PASSWORD|SECRET/i.test(name)));
  return {
    modelSettings: { root, home, engine, python, environment, hfHome: env.HF_HOME },
    setup: { configuration_path: publicPath(configurationPath), server_config: publicPath(serverConfig), llm_configured: llmConfigured,
      llm_message: llmConfigured ? 'LLM 配置已读取，密钥仅用于主服务。' : 'LLM 尚未就绪。启动主服务后进入“LLM 接入”，填写接口格式、地址、模型和密钥。',
      windows_client_path: './target/windows-client' },
    definitions: [
      { id: 'server', url: serverUrl, issue: serverIssue, cwd: root, command: 'cargo',
        args: ['run', '--locked', '-p', 'meowlive-server', '--', '--config', serverConfig], env: serverEnv,
        logPath: join(root, 'logs/control-panel/server.log') },
      { id: 'tts', url: ttsUrl, issue: ttsIssue, cwd: root, command: python, memoryGuard: true,
        args: [join(root, 'scripts/start-managed-inference.py'), '--engine-root', engine, '--data-dir', join(root, 'data/control-panel-inference'),
          '--port', new URL(ttsUrl).port, '--device', settings.ttsDevice, '--memory-mode', settings.ttsMemoryMode], env: { ...ttsEnv, PYTHONDONTWRITEBYTECODE: '1', PYTHONUNBUFFERED: '1' },
        logPath: join(root, 'logs/control-panel/tts.log') },
    ],
  };
}
