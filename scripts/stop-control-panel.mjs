#!/usr/bin/env node
import { fileURLToPath } from 'node:url';
import { stopProjectProcesses } from './launcher/process-cleanup.mjs';

if (process.platform !== 'linux' || process.argv.length > 2) {
  console.error('请在 Linux / WSL 的项目目录运行 npm run stop。');
  process.exitCode = 1;
} else {
  try {
    const result = await stopProjectProcesses(fileURLToPath(new URL('../', import.meta.url)));
    if (result.remaining.length) {
      console.error(`仍有进程未退出：${result.remaining.join('、')}。请稍后重试 npm run stop。`);
      process.exitCode = 1;
    } else {
      console.log('本项目控制面板、主服务和受管 TTS 已停止；已向受管 Windows 执行端发送停止请求。可以重新 npm start。');
    }
  } catch (error) {
    console.error(`清理失败（${error.code ?? error.message}），请检查当前用户的进程与目录权限。`);
    process.exitCode = 1;
  }
}
