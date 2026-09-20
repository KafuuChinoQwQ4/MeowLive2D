import { createConnection } from 'node:net';

async function portOccupied(url) {
  return new Promise(resolve => {
    const address = new URL(url);
    const socket = createConnection({ host: address.hostname, port: Number(address.port) });
    const finish = value => { socket.destroy(); resolve(value); };
    socket.once('connect', () => finish(true));
    socket.once('error', () => finish(false));
    socket.setTimeout(300, () => finish(true));
  });
}

export async function inspectEndpoint(definition) {
  try {
    const path = definition.id === 'tts' ? '/openapi.json' : '/api/health';
    const response = await fetch(`${definition.url}${path}`, { signal: AbortSignal.timeout(800), redirect: 'error' });
    let bytes = 0;
    const chunks = [];
    if (!response.ok) { await response.body?.cancel(); throw new Error('unhealthy'); }
    for await (const chunk of response.body) {
      bytes += chunk.length;
      if (bytes > 512 * 1024) throw new Error('too large');
      chunks.push(chunk);
    }
    const data = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    const healthy = definition.id === 'tts'
      ? typeof data.openapi === 'string' && Boolean(data.paths?.['/tts']?.post)
      : data.protocol_version === 3 && data.service === 'meowlive' && typeof data.bridge_connected === 'boolean';
    return { healthy, occupied: true };
  } catch { return { healthy: false, occupied: await portOccupied(definition.url) }; }
}
