import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { networkInterfaces, hostname as osHostname } from "node:os";
import { log } from "./logger.js";

/**
 * Resolve the friendly mDNS hostname advertised by macOS (e.g. weichaoyingdeMac-mini.local).
 * Falls back to os.hostname() with a .local suffix on non-macOS.
 */
export function getLocalHostname(): string {
  if (process.platform === "darwin") {
    try {
      const out = execFileSync("scutil", ["--get", "LocalHostName"], {
        encoding: "utf8",
      }).trim();
      if (out) return `${out}.local`;
    } catch {
      // fall through
    }
  }
  const base = osHostname().replace(/\.(local|lan)$/i, "");
  return `${base}.local`;
}

function isPrivateLan(ip: string): boolean {
  const parts = ip.split(".").map(Number);
  if (parts.length !== 4) return false;
  const [a, b] = parts;
  if (a === 10) return true;
  if (a === 172 && b >= 16 && b <= 31) return true;
  if (a === 192 && b === 168) return true;
  return false;
}

/** All non-internal IPv4 LAN addresses (private ranges only). */
export function getLanIps(): string[] {
  const out = new Set<string>();
  for (const ifaces of Object.values(networkInterfaces())) {
    for (const iface of ifaces ?? []) {
      if (iface.family !== "IPv4" || iface.internal) continue;
      if (isPrivateLan(iface.address)) out.add(iface.address);
    }
  }
  return [...out].sort();
}

let mdnsChild: ChildProcess | null = null;

/**
 * Register a Bonjour service via the macOS native `dns-sd` daemon so Android NSD
 * can discover the server by name. Uses the system's own mDNS hostname, so no
 * duplicate A records are advertised and macOS will not rename LocalHostName.
 */
export function publishBonjourService(name: string, type: string, port: number): void {
  if (process.platform !== "darwin") {
    log({ msg: "bonjour skipped (macOS only)", platform: process.platform });
    return;
  }
  try {
    const child = spawn("dns-sd", ["-R", name, `_${type}._tcp`, "local", String(port)], {
      stdio: "ignore",
    });
    mdnsChild = child;
    child.on("error", (err) => {
      log({ msg: "bonjour failed (continuing without it)", error: err.message });
      mdnsChild = null;
    });
    child.on("exit", (code) => {
      if (code !== 0) log({ msg: "bonjour exited", code: code ?? "?" });
      mdnsChild = null;
    });
    log({ msg: "bonjour service registered", service: `${type}.local`, name, port });
  } catch (err) {
    log({ msg: "bonjour publish failed (continuing without it)", error: err instanceof Error ? err.message : String(err) });
  }
}

export function stopBonjour(): void {
  try {
    mdnsChild?.kill();
  } catch {
    // ignore
  }
  mdnsChild = null;
}
