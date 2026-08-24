// 请求超时毫秒数
const FETCH_TIMEOUT_MS = 10_000;

/**
 * 带超时的 JSON GET 请求。网络失败、超时、非 2xx 或响应非 JSON 时抛出，
 * 由调用方决定降级行为
 */
export async function fetchJson(url: string): Promise<unknown> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const response = await fetch(url, { signal: controller.signal });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    return await response.json();
  } finally {
    clearTimeout(timer);
  }
}
