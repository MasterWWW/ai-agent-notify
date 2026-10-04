import { error } from "../core/logger.js";
import { getHealth, resolveBaseUrl } from "../transport/client.js";
import { argValue } from "./util.js";

export async function cmdStatus(argv: string[]): Promise<number> {
  const baseUrl = resolveBaseUrl(argValue(argv, "--base-url"));
  try {
    const health = await getHealth(baseUrl);
    console.log(`status=${health.status} version=${health.version ?? "?"} base=${baseUrl}`);
    return 0;
  } catch (err) {
    error("server not reachable", err);
    return 1;
  }
}
