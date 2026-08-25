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

/** 并发请求多个 JSON 地址，返回第一个请求成功且通过解析校验的结果 */
export async function fetchFirstValidJson<T>(urls: readonly string[], parse: (data: unknown) => T): Promise<T> {
  if (urls.length === 0) {
    throw new Error("没有可用的数据源");
  }

  return new Promise<T>((resolve, reject) => {
    let pending = urls.length;
    let firstError: unknown;

    for (const url of urls) {
      void fetchJson(url)
        .then(parse)
        .then(resolve)
        .catch((cause: unknown) => {
          firstError ??= cause;
          pending -= 1;
          if (pending === 0) {
            reject(firstError instanceof Error ? firstError : new Error("所有数据源均获取失败"));
          }
        });
    }
  });
}
