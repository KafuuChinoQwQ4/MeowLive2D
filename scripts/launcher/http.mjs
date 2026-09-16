import { timingSafeEqual } from 'node:crypto';

function json(response, status, value) {
  response.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' });
  response.end(JSON.stringify(value));
}

function readBody(request) {
  return new Promise((resolve, reject) => {
    let size = 0;
    const chunks = [];
    const fail = status => finish(Object.assign(new Error('invalid body'), { status }));
    const timer = setTimeout(() => fail(408), 3000);
    function finish(error, value) {
      clearTimeout(timer);
      request.off('data', data); request.off('end', end); request.off('error', aborted); request.off('aborted', aborted);
      if (error) { request.resume(); reject(error); } else resolve(value);
    }
    function data(chunk) { size += chunk.length; if (size > 1024) fail(413); else chunks.push(chunk); }
    function end() {
      try { finish(null, JSON.parse(Buffer.concat(chunks).toString('utf8'))); } catch { fail(400); }
    }
    function aborted() { fail(400); }
    request.on('data', data); request.once('end', end); request.once('error', aborted); request.once('aborted', aborted);
  });
}

export function createLauncherMiddleware(manager, port = 1420, models) {
  const hosts = new Set([`127.0.0.1:${port}`, `localhost:${port}`]);
  return (request, response, next) => {
    const path = request.url?.split('?')[0];
    if (!path?.startsWith('/api/launcher/')) return next();
    void (async () => {
      const origin = request.headers.origin;
      if (!hosts.has(request.headers.host) || (origin && origin !== `http://${request.headers.host}`)) {
        json(response, 403, { message: '启动管理仅允许从本机控制面板访问。' }); return;
      }
      if (path === '/api/launcher/status' && request.method === 'GET') {
        json(response, 200, await manager.snapshot()); return;
      }
      if (models && path === '/api/launcher/models' && request.method === 'GET') {
        json(response, 200, await models.snapshot()); return;
      }
      const match = /^\/api\/launcher\/services\/(server|tts|windows)$/.exec(path);
      const modelMatch = models && /^\/api\/launcher\/models\/(scan|select|download|cancel)$/.exec(path);
      if ((!match && !modelMatch) || request.method !== 'POST') { json(response, 404, { message: '未知启动管理接口。' }); return; }
      const token = request.headers['x-meowlive-launcher-token'];
      if (!origin || typeof token !== 'string' || Buffer.byteLength(token) !== Buffer.byteLength(manager.token)
          || !timingSafeEqual(Buffer.from(token), Buffer.from(manager.token))) {
        json(response, 403, { message: '启动管理会话已失效，请刷新页面后重试。' }); return;
      }
      if (request.headers['content-type']?.split(';')[0].trim() !== 'application/json') {
        request.resume(); json(response, 415, { message: '启停请求必须为 JSON。' }); return;
      }
      const body = await readBody(request);
      if (modelMatch) {
        const keys = body && typeof body === 'object' && !Array.isArray(body) ? Object.keys(body) : null;
        const valid = keys && (modelMatch[1] === 'scan' ? keys.length === 0
          : keys.length === 1 && keys[0] === 'id' && typeof body.id === 'string' && /^[a-zA-Z0-9-]{1,80}$/.test(body.id));
        if (!valid) { json(response, 400, { message: '模型操作参数无效。' }); return; }
        try { await models.action(modelMatch[1], body.id); }
        catch (error) { json(response, 409, { message: error.message }); return; }
        json(response, 200, await models.snapshot()); return;
      }
      if (!body || typeof body !== 'object' || Array.isArray(body) || Object.keys(body).length !== 1 || typeof body.enabled !== 'boolean') {
        json(response, 400, { message: '启停请求只允许包含 enabled 布尔值。' }); return;
      }
      try { await manager.setEnabled(match[1], body.enabled); }
      catch (error) { json(response, 409, { message: error.message }); return; }
      json(response, 200, await manager.snapshot());
    })().catch(error => {
      if (!response.headersSent) json(response, error.status ?? 500, { message: error.status === 413 ? '启停请求过大。' : '启动管理请求失败，请刷新状态后重试。' });
    });
  };
}
