const BASE = "https://open.feishu.cn/open-apis";

let tokenCache: { token: string; expiresAt: number } | null = null;

/**
 * Fetch and cache the tenant_access_token (2h TTL, refreshed before expiry).
 * The cache is process-wide and shared by all Feishu callers.
 */
export async function getTenantToken(appId: string, appSecret: string): Promise<string> {
  if (tokenCache && Date.now() < tokenCache.expiresAt - 60_000) return tokenCache.token;
  const res = await fetch(`${BASE}/auth/v3/tenant_access_token/internal`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ app_id: appId, app_secret: appSecret }),
  });
  const data = (await res.json().catch(() => ({}))) as {
    code?: number;
    msg?: string;
    tenant_access_token?: string;
    expire?: number;
  };
  if (data.code !== 0 || !data.tenant_access_token) {
    throw new Error(`获取 tenant_access_token 失败: code=${data.code} msg=${data.msg ?? ""}`);
  }
  tokenCache = {
    token: data.tenant_access_token,
    expiresAt: Date.now() + (data.expire ?? 7200) * 1000,
  };
  return tokenCache.token;
}

/** Drop the cached token (used when a call reports an expired/invalid token). */
export function resetTokenCache(): void {
  tokenCache = null;
}

export const FEISHU_BASE = BASE;
