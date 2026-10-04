// Minimal structured logger. Never logs tokens, secrets, or full payloads.
function ts(): string {
  const d = new Date();
  const p = (n: number, l = 2) => String(n).padStart(l, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}

export function log(fields: Record<string, string | number | undefined>): void {
  const parts: string[] = [ts()];
  for (const [k, v] of Object.entries(fields)) {
    if (v === undefined) continue;
    parts.push(`${k}=${v}`);
  }
  console.log(parts.join(" "));
}

export function error(msg: string, err?: unknown): void {
  const detail = err instanceof Error ? err.message : String(err ?? "");
  console.error(`${ts()} error="${msg}" ${detail ? `detail="${detail}"` : ""}`);
}
