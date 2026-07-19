export function normalizeServerUrl(value: string): string {
  const text = value.trim().replace(/\/+$/, '');
  let parsed: URL;
  try {
    parsed = new URL(text);
  } catch {
    throw new Error('服务器地址无效');
  }
  if (parsed.username || parsed.password || parsed.search || parsed.hash) {
    throw new Error('服务器地址不能包含凭据、查询参数或片段');
  }
  const host = parsed.hostname.toLowerCase();
  const loopback = host === 'localhost' || host === '[::1]' || host === '::1'
    || /^127(?:\.\d{1,3}){3}$/.test(host);
  if (parsed.protocol === 'http:' && !loopback) {
    throw new Error('远程服务器必须使用 HTTPS');
  }
  if (parsed.protocol !== 'https:' && !(parsed.protocol === 'http:' && loopback)) {
    throw new Error('仅允许 HTTPS，回环地址可使用 HTTP');
  }
  return text;
}
