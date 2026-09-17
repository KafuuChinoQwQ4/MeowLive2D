export function installShutdownHandlers(shutdown) {
  let completion;
  const stop = () => {
    completion ??= Promise.resolve().then(shutdown).catch(() => {
      console.error('服务清理失败，请在项目目录运行 npm run stop 后重试。');
      process.exitCode = 1;
    });
  };
  // npm forwards terminal signals to its child, so one Ctrl+C can arrive twice.
  // Keep the handlers installed until exit; a second signal must not abandon cleanup.
  for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, stop);
}
