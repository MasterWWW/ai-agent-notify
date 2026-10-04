#!/usr/bin/env node
/**
 * 权限请求交互（飞书卡片远程控制）非交互验收脚本。
 * 覆盖计划书「七、验收清单」中不需要真人点击/真实飞书的项。
 *
 * 前置：pnpm build（需要 server/dist）
 * 运行：node scripts/verify-permission.mjs
 * 说明：不发送真实飞书卡片（测试配置无 App Secret / 无绑定目标），无网络依赖。
 */
import { existsSync } from "node:fs";
import { spawn } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const DIST = join(ROOT, "server", "dist");
if (!existsSync(join(DIST, "index.js"))) {
  console.error("server/dist 不存在，请先运行 pnpm build");
  process.exit(1);
}

const { startApp } = await import(join(DIST, "transport", "server.js"));
const { PermissionStore } = await import(join(DIST, "domain", "permission.js"));
const { PermissionService } = await import(join(DIST, "features", "permission.js"));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let ok = true;
const check = (name, cond, extra = "") => {
  console.log(`${cond ? "PASS" : "FAIL"}  ${name}${extra ? "  " + extra : ""}`);
  if (!cond) ok = false;
};

// 测试配置：绑定一个"本人" open_id，但没有 App Secret → 不会发真实卡片
const ownerOpenId = "ou_test_owner";
const cfg = { feishuOpenId: ownerOpenId };
const service = new PermissionService(() => cfg);
const token = "test-token";
const app = await startApp({ host: "127.0.0.1", port: 0, token, mdns: false }, () => [], service);
const base = `http://127.0.0.1:${app.port}`;

async function post(body, tok = token) {
  const res = await fetch(`${base}/api/permission-requests`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${tok}` },
    body: JSON.stringify(body),
  });
  return { status: res.status, body: await res.json() };
}
async function wait(id, ms, tok = token) {
  const res = await fetch(`${base}/api/permission-requests/${id}/wait?timeout=${ms}`, {
    headers: { Authorization: `Bearer ${tok}` },
  });
  return { status: res.status, body: await res.json() };
}
const cardAction = (rid, act, operatorOpenId, messageId = "mock_msg") => ({
  messageId,
  chatId: "oc_mock",
  operator: { openId: operatorOpenId },
  action: { tag: "button", value: { rid, act } },
});

// S1: Store 基础
{
  const store = new PermissionStore({ ttlMs: 60_000 });
  const req = store.create({ agent: "codex", toolName: "Bash", command: "ls", cwd: "/tmp", project: "tmp" });
  check("S1 create/get", store.get(req.id)?.status === "pending");
  store.resolve(req.id, "deny");
  check("S1 resolve", store.get(req.id)?.status === "denied");
  store.stop();
}

// S2: HTTP 鉴权
{
  const r1 = await post({ agent: "codex", toolName: "Bash" }, "wrong");
  check("S2 无/错 token -> 401", r1.status === 401);
  const r2 = await wait("pr_xxx", 1000, "wrong");
  check("S2 wait 无 token -> 401", r2.status === 401);
}

// S3: 创建 + 无人决定 -> timeout（无卡片发送）
{
  const r = await post({ agent: "codex", toolName: "Bash", command: "echo T1", cwd: "/tmp" });
  check("S3 POST 创建", r.status === 200 && r.body.success === true);
  const w = await wait(r.body.requestId, 2000);
  check("S3 无人决定 -> timeout", w.body.status === "timeout");
}

// S4: 本人点允许 -> allowed
{
  const r = await post({ agent: "claude", toolName: "Bash", command: "pnpm add lodash", cwd: "/tmp" });
  await service.handleCardAction(cardAction(r.body.requestId, "allow", ownerOpenId));
  const w = await wait(r.body.requestId, 2000);
  check("S4 本人允许 -> allowed", w.body.status === "allowed" && w.body.decision === "allow");
}

// S5: 非本人点击 -> 忽略
{
  const req = service.store.create({ agent: "codex", toolName: "Bash", command: "echo T3", cwd: "/tmp" });
  await service.handleCardAction(cardAction(req.id, "deny", "ou_wrong_user"));
  const w = await wait(req.id, 1200);
  check("S5 非本人忽略 -> timeout", w.body.status === "timeout");
}

// S6: 文本兜底「拒绝 <id>」
{
  const req = service.store.create({ agent: "claude", toolName: "Bash", command: "rm -rf /tmp/x", cwd: "/tmp" });
  const msg = await service.resolveFromText(req.id, "deny", ownerOpenId);
  const w = await wait(req.id, 1200);
  check("S6 文本拒绝生效", w.body.status === "denied" && w.body.decision === "deny" && msg.includes("已拒绝"), msg);
}

// S7: Store TTL 超时回调
{
  let timedOut = null;
  const store = new PermissionStore({ ttlMs: 200, sweepMs: 80, onTimeout: (req) => (timedOut = req.id) });
  const req = store.create({ agent: "codex", toolName: "Bash", command: "x", cwd: "/tmp" });
  await sleep(700);
  check("S7 超时回调 + 状态", timedOut === req.id && store.get(req.id)?.status === "timeout");
  store.stop();
}

// S8: 未绑定发送目标（feishuOpenId 缺失）-> 创建仍成功（卡片跳过，不影响请求）
{
  const svc2 = new PermissionService(() => ({}));
  const r = await svc2.create({ agent: "codex", toolName: "Bash", command: "x", cwd: "/tmp" });
  check("S8 未配置目标 -> 创建成功", !!r.requestId);
  svc2.stop();
}

// H1: 阻断式 Hook 全链路 allow（spawn 真实 CLI，走 HTTP）
{
  const payload = JSON.stringify({ tool_name: "Bash", tool_input: { command: "pnpm add axios" }, cwd: "/tmp", session_id: "s1" });
  const p = new Promise((resolvePromise) => {
    const child = spawn("node", [join(DIST, "index.js"), "hook", "codex-permission", "--wait", "--token", token, "--base-url", base]);
    let out = "";
    child.stdout.on("data", (d) => (out += d));
    child.on("close", (code) => resolvePromise({ code, out: out.trim() }));
    child.stdin.write(payload);
    child.stdin.end();
  });
  let rid;
  for (let i = 0; i < 50 && !rid; i++) {
    await sleep(150);
    // 按命令精确匹配 H1 的请求（避免命中前面测试残留的 pending 请求）
    const pend = service.store.listPending().filter((r) => r.command?.includes("axios"));
    if (pend.length) rid = pend[0].id;
  }
  check("H1 请求进入 Server", !!rid);
  if (rid) await service.resolveFromText(rid, "allow", ownerOpenId);
  const r = await p;
  check("H1 Hook 输出 allow JSON", r.code === 0 && r.out.includes('"behavior":"allow"'), r.out.slice(0, 100));
}

// H3: Server 不可达 -> 静默 exit 0
{
  const payload = JSON.stringify({ tool_name: "Bash", tool_input: { command: "touch /tmp/h3" }, cwd: "/tmp" });
  const r = await new Promise((resolvePromise) => {
    const child = spawn("node", [join(DIST, "index.js"), "hook", "codex-permission", "--wait", "--token", "t", "--base-url", "http://127.0.0.1:1"]);
    let out = "";
    child.stdout.on("data", (d) => (out += d));
    child.stderr.on("data", (d) => (out += d));
    child.on("close", (code) => resolvePromise({ code, out: out.trim() }));
    child.stdin.write(payload);
    child.stdin.end();
  });
  check("H3 Server 挂 -> 静默 exit 0", r.code === 0 && r.out === "");
}

// D1/D2: 安全命令自动放行 / 危险命令不自动放行
{
  const run = (payload) =>
    new Promise((resolvePromise) => {
      const child = spawn("node", [join(DIST, "index.js"), "hook", "claude-permission", "--wait", "--token", "t", "--base-url", "http://127.0.0.1:1"]);
      let out = "";
      child.stdout.on("data", (d) => (out += d));
      child.stderr.on("data", (d) => (out += d));
      child.on("close", (code) => resolvePromise({ code, out: out.trim() }));
      child.stdin.write(payload);
      child.stdin.end();
    });
  const safe = await run(JSON.stringify({ tool_name: "Bash", tool_input: { command: "git status" }, cwd: "/tmp" }));
  check("D1 git status 自动放行", safe.code === 0 && safe.out.includes('"behavior":"allow"'));
  const risky = await run(JSON.stringify({ tool_name: "Bash", tool_input: { command: "rm -rf /tmp/x" }, cwd: "/tmp" }));
  check("D2 rm 不自动放行（静默）", risky.code === 0 && risky.out === "");
}

await app.close();
service.stop();
console.log(ok ? "\nALL PASS" : "\nSOME FAILED");
process.exit(ok ? 0 : 1);
