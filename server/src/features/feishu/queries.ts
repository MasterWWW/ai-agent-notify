import { getTenantToken, FEISHU_BASE } from "./api.js";

/** 通过手机号查用户的 open_id（用于发到与机器人的单聊）。需要 contact:user.base:readonly 权限。 */
export async function resolveOpenIdByMobile(
  appId: string,
  appSecret: string,
  mobile: string
): Promise<string> {
  const token = await getTenantToken(appId, appSecret);
  const res = await fetch(`${FEISHU_BASE}/contact/v3/users/batch_get_id?user_id_type=open_id`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({ mobiles: [mobile] }),
  });
  const data = (await res.json().catch(() => ({}))) as {
    code?: number;
    msg?: string;
    data?: { user_list?: Array<{ user_id?: string; mobile?: string }> };
  };
  if (data.code !== 0) throw new Error(`code=${data.code} msg=${data.msg ?? ""}`);
  const user = data.data?.user_list?.[0];
  if (!user?.user_id) throw new Error("没有找到该手机号对应的用户（请确认手机号在组织通讯录内）");
  return user.user_id;
}

/** 列出机器人所在的群/会话（用于让用户挑选 chat_id）。需要 im:chat:readonly 权限。 */
export async function listChats(
  appId: string,
  appSecret: string
): Promise<Array<{ chat_id: string; name: string }>> {
  const token = await getTenantToken(appId, appSecret);
  const res = await fetch(`${FEISHU_BASE}/im/v1/chats?page_size=50`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  const data = (await res.json().catch(() => ({}))) as {
    code?: number;
    msg?: string;
    data?: { items?: Array<{ chat_id: string; name?: string }> };
  };
  if (data.code !== 0) throw new Error(`code=${data.code} msg=${data.msg ?? ""}`);
  return (data.data?.items ?? []).map((c) => ({ chat_id: c.chat_id, name: c.name ?? "(未命名群)" }));
}
