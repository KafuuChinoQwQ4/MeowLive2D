#!/usr/bin/env node
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { createServer } from 'vite';
import { loadConfiguration } from './launcher/config.mjs';
import { Supervisor } from './launcher/supervisor.mjs';
import { createLauncherMiddleware } from './launcher/http.mjs';
import { ModelLibrary } from './launcher/model-library.mjs';
import { closeModelConnections } from './launcher/model-download.mjs';
import { WindowsClientSupervisor, managedServices } from './launcher/windows-client.mjs';
import { installShutdownHandlers } from './launcher/shutdown.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2);
if (args.some(arg => arg !== '--no-open')) { console.error('用法：npm start [-- --no-open]'); process.exit(1); }
if (process.platform !== 'linux') { console.error('Windows 请双击 launchers/start-windows.cmd；Linux / WSL2 请运行 ./launchers/start.sh。'); process.exit(1); }
const configuration = await loadConfiguration(root);
const supervisor = new Supervisor(configuration);
const models = new ModelLibrary(configuration, supervisor);
await models.initialize();
const windows = await new WindowsClientSupervisor(configuration).initialize();
const services = managedServices(supervisor, windows);
let vite, stopping = false;
let cleanupPromise;
function cleanup() {
  cleanupPromise ??= Promise.all([models.close(), windows.close().then(() => supervisor.close())]).then(() => closeModelConnections());
  return cleanupPromise;
}
async function shutdown() {
  if (stopping) return;
  stopping = true;
  console.log('\n正在关闭控制面板并回收本次启动的服务…');
  await cleanup();
  await vite?.close();
}
installShutdownHandlers(shutdown);
try {
  vite = await createServer({
    root: join(root, 'apps/desktop'), configFile: join(root, 'apps/desktop/vite.config.ts'),
    define: {
      'import.meta.env.VITE_MEOWLIVE_LAUNCHER': JSON.stringify('true'),
      'import.meta.env.VITE_MEOWLIVE_SERVER_URL': JSON.stringify(configuration.definitions[0].url),
    },
    plugins: [{ name: 'meowlive-local-launcher',
      configureServer(server) { server.middlewares.use(createLauncherMiddleware(services, 1420, models)); },
      // Vite also handles SIGTERM and exits after close(). Join its close hook so
      // that detached TTS children are reaped before Vite terminates the process.
      closeServer({ reason }) { if (reason !== 'restart') return cleanup(); },
    }],
    server: { host: '127.0.0.1', port: 1420, strictPort: true, open: !args.includes('--no-open'),
      fs: { strict: true, allow: [join(root, 'apps/desktop'), join(root, 'packages/contracts'), join(root, 'node_modules')],
        deny: ['**/.git/**', '**/.env*', '**/docs/**', '**/config/**', '**/logs/**', '**/data/**', '**/*.{crt,pem,key}'] } },
  });
  await vite.listen();
  void services.initialize().catch(() => console.error('服务自动启动未完成，请在“启动与运行”查看状态与日志。'));
  console.log('\nMeowLive2D 控制面板：http://127.0.0.1:1420');
  console.log('主服务正在自动启动，就绪后会自动启动 TTS 和 Windows 执行端。缺少依赖或模型时请在面板按提示补齐。保持启动窗口打开。\n');
} catch (error) {
  console.error(error.code === 'EADDRINUSE' || /already in use/.test(error.message)
    ? '1420 端口已被占用。已有控制面板可直接打开；若此前运行了 npm run dev，请先在原终端 Ctrl+C。'
    : '控制面板启动失败，请检查依赖是否安装、端口是否可用。');
  await shutdown();
  process.exitCode = 1;
}
