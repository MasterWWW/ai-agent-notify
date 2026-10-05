import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, FeishuConfigInput, StatusInfo } from "./types";

type FeishuMode = "app" | "webhook";

const POLL_MS = 2000;

export default function App() {
  const [status, setStatus] = useState<StatusInfo | null>(null);
  const [mode, setMode] = useState<FeishuMode>("app");
  const [appId, setAppId] = useState("");
  const [appSecret, setAppSecret] = useState("");
  const [chatId, setChatId] = useState("");
  const [webhook, setWebhook] = useState("");
  const [webhookSecret, setWebhookSecret] = useState("");
  const [saved, setSaved] = useState(false);
  const [feishuResult, setFeishuResult] = useState("");
  const [busy, setBusy] = useState(false);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await invoke<StatusInfo>("get_status"));
    } catch (e) {
      console.error("get_status failed", e);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
    const t = window.setInterval(refreshStatus, POLL_MS);
    return () => window.clearInterval(t);
  }, [refreshStatus]);

  useEffect(() => {
    void (async () => {
      try {
        const cfg = await invoke<AppConfig>("get_config");
        if (cfg.feishuAppId) setAppId(cfg.feishuAppId);
        if (cfg.feishuAppSecret) setAppSecret(cfg.feishuAppSecret);
        if (cfg.feishuChatId) setChatId(cfg.feishuChatId);
        if (cfg.feishuWebhook) setWebhook(cfg.feishuWebhook);
        if (cfg.feishuSecret) setWebhookSecret(cfg.feishuSecret);
        if (cfg.feishuAppId) setMode("app");
        else if (cfg.feishuWebhook) setMode("webhook");
      } catch (e) {
        console.error("get_config failed", e);
      }
    })();
  }, []);

  const toggleServer = useCallback(async () => {
    setBusy(true);
    try {
      if (status?.running) {
        await invoke("stop_server");
      } else {
        await invoke("start_server", { port: 3210 });
      }
      await refreshStatus();
    } catch (e) {
      setFeishuResult(`⚠️ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }, [status, refreshStatus]);

  const copyConnectUrl = useCallback(() => {
    if (!status) return;
    const url = `ws://${status.hostname}:${status.port}/ws?token=${status.token}`;
    void invoke("copy_text", { text: url });
  }, [status]);

  const copyToken = useCallback(() => {
    if (!status) return;
    void invoke("copy_text", { text: status.token });
  }, [status]);

  const saveFeishu = useCallback(async () => {
    setBusy(true);
    setSaved(false);
    try {
      const input: FeishuConfigInput =
        mode === "app"
          ? {
              feishu_app_id: appId,
              feishu_app_secret: appSecret,
              feishu_chat_id: chatId,
            }
          : { feishu_webhook: webhook, feishu_secret: webhookSecret };
      await invoke("save_feishu_config", { input });
      setSaved(true);
      setFeishuResult("");
      await refreshStatus();
    } catch (e) {
      setFeishuResult(`⚠️ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }, [mode, appId, appSecret, chatId, webhook, webhookSecret, refreshStatus]);

  const runFeishu = useCallback(async (cmd: "feishu_test" | "feishu_chats") => {
    setBusy(true);
    setFeishuResult("正在执行…");
    try {
      setFeishuResult(await invoke<string>(cmd));
    } catch (e) {
      setFeishuResult(`⚠️ ${String(e)}`);
    } finally {
      setBusy(false);
    }
  }, []);

  const openStateDir = useCallback(() => {
    void invoke("open_state_dir");
  }, []);

  const quitApp = useCallback(() => {
    void invoke("quit_app");
  }, []);

  const running = status?.running ?? false;
  const feishuConfigured = status?.feishu_configured ?? false;
  const botConnected = status?.bot_connected ?? false;

  return (
    <div className="app">
      <header className="status-row">
        <span className={`dot ${running ? "on" : "off"}`} />
        <div className="status-text">
          <span className="title">
            {running ? `运行中 · 端口 ${status?.port}` : "已停止"}
          </span>
          <span className={`sub ${botConnected ? "ok" : ""}`}>
            {running
              ? botConnected
                ? "飞书长连接已连接"
                : "飞书长连接未连接"
              : "本地服务未启动"}
          </span>
        </div>
        <button className="btn" disabled={busy} onClick={toggleServer}>
          {running ? "停止" : "启动"}
        </button>
      </header>

      <p className="hostname">主机名：{status?.hostname ?? "…"}</p>
      <div className="btn-row">
        <button className="btn" disabled={!running} onClick={copyConnectUrl}>
          复制连接地址
        </button>
        <button className="btn" onClick={copyToken}>
          复制 Token
        </button>
      </div>

      <hr />

      <section className="feishu">
        <div className="segmented">
          <button
            className={mode === "app" ? "active" : ""}
            onClick={() => setMode("app")}
          >
            自建应用机器人
          </button>
          <button
            className={mode === "webhook" ? "active" : ""}
            onClick={() => setMode("webhook")}
          >
            Webhook 机器人
          </button>
        </div>

        {mode === "app" ? (
          <>
            <input
              className="field"
              placeholder="App ID（cli_xxx）"
              value={appId}
              onChange={(e) => setAppId(e.target.value)}
            />
            <input
              className="field"
              type="password"
              placeholder="App Secret"
              value={appSecret}
              onChange={(e) => setAppSecret(e.target.value)}
            />
            <input
              className="field"
              placeholder="单聊/群 Chat ID（oc_xxx）"
              value={chatId}
              onChange={(e) => setChatId(e.target.value)}
            />
            <p className="hint">
              获取方式：飞书里和机器人单聊 → 右上角 ⋯ → 群设置 → 查看群 ID（单聊/群聊都支持）
            </p>
          </>
        ) : (
          <>
            <input
              className="field"
              placeholder="Webhook 地址"
              value={webhook}
              onChange={(e) => setWebhook(e.target.value)}
            />
            <input
              className="field"
              type="password"
              placeholder="安全密钥（可选）"
              value={webhookSecret}
              onChange={(e) => setWebhookSecret(e.target.value)}
            />
          </>
        )}

        <div className="btn-row">
          <span className={`save-state ${saved ? "ok" : feishuConfigured ? "ok" : ""}`}>
            {saved ? "已保存 ✓" : feishuConfigured ? "已配置 ✅" : "未配置"}
          </span>
          <div className="spacer" />
          {mode === "app" ? (
            <button className="btn" disabled={busy} onClick={() => runFeishu("feishu_chats")}>
              查群列表
            </button>
          ) : null}
          <button className="btn" disabled={busy} onClick={() => runFeishu("feishu_test")}>
            测试发送
          </button>
          <button className="btn primary" disabled={busy} onClick={saveFeishu}>
            保存
          </button>
        </div>

        {feishuResult ? <pre className="result">{feishuResult}</pre> : null}
      </section>

      <hr />

      <section className="events">
        <h2>最近事件</h2>
        {status && status.recent_events.length > 0 ? (
          <ul>
            {status.recent_events.map((line, i) => (
              <li key={i}>{line}</li>
            ))}
          </ul>
        ) : (
          <p className="empty">（暂无）</p>
        )}
      </section>

      <hr />

      <footer>
        <button className="btn" onClick={openStateDir}>
          打开数据目录
        </button>
        <div className="spacer" />
        <button className="btn danger" onClick={quitApp}>
          退出
        </button>
      </footer>
    </div>
  );
}
