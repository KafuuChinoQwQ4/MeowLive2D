import { createWriteStream } from 'node:fs';

export function captureOutput(child, definition, onDiagnostic) {
  const stream = createWriteStream(definition.logPath, { flags: 'w', mode: 0o600 });
  const secrets = Object.entries(definition.env).filter(([key, value]) => /KEY|TOKEN|PASSWORD|SECRET/i.test(key) && value?.length >= 4).map(([, value]) => value);
  let written = 0;
  stream.on('error', () => onDiagnostic('无法写入运行日志，请检查 logs/control-panel 目录权限。'));
  function line(text) {
    if (/CUDA out of memory/i.test(text)) onDiagnostic('GPU 显存不足。请关闭占用显存的其他程序，再重新启动 TTS。');
    if (/No module named/i.test(text)) onDiagnostic('Python 环境缺少依赖，请检查 ttsPython 是否指向完整的 GPT-SoVITS 环境。');
    if (/Address already in use|地址已在使用/i.test(text)) onDiagnostic('服务端口已被占用，请先停止占用该端口的程序。');
    let safe = text;
    for (const secret of secrets) safe = safe.replaceAll(secret, '[REDACTED]');
    safe = safe.replace(/sk-[A-Za-z0-9_-]+/g, '[REDACTED]');
    if (written < 2 * 1024 * 1024) { written += Buffer.byteLength(safe); stream.write(safe); }
  }
  for (const pipe of [child.stdout, child.stderr]) {
    let pending = '';
    pipe.setEncoding('utf8');
    pipe.on('data', chunk => {
      pending += chunk;
      const lines = pending.split('\n'); pending = lines.pop();
      for (const value of lines) line(`${value}\n`);
      if (pending.length > 65536) { pending = ''; line('[过长的日志行已省略]\n'); }
    });
    pipe.on('end', () => { if (pending) line(`${pending}\n`); });
  }
  child.once('close', () => stream.end());
}
