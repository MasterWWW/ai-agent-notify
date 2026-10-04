import SwiftUI
import AppKit

// MARK: - Menu bar popover content

/// UI layer: renders state and collects Feishu config. Owns no process
/// management and no config-file details (those live in ServerController /
/// AppConfigFile).
struct AppMenuView: View {
    @ObservedObject var controller: ServerController
    @State private var mode = "app"
    @State private var appId = ""
    @State private var appSecret = ""
    @State private var chatId = ""
    @State private var webhook = ""
    @State private var webhookSecret = ""
    @State private var saved = false

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Group {
                HStack(spacing: 8) {
                    Circle()
                        .fill(controller.isRunning ? Color.green : Color.gray)
                        .frame(width: 8, height: 8)
                    Text(controller.isRunning ? "运行中 · 端口 \(controller.port)" : "已停止")
                        .font(.headline)
                    Spacer()
                    if controller.isRunning {
                        Button("停止") { controller.stop() }
                    } else {
                        Button("启动") { controller.start() }
                    }
                }
                Text("主机名：\(controller.hostname)").font(.caption).foregroundColor(.secondary)
                HStack(spacing: 8) {
                    Button("复制连接地址") { controller.copyConnectURL() }
                    Button("复制 Token") { controller.copyToken() }
                }
            }

            Divider()

            Group {
                Picker("接入方式", selection: $mode) {
                    Text("自建应用机器人").tag("app")
                    Text("Webhook 机器人").tag("webhook")
                }
                .pickerStyle(.segmented)

                if mode == "app" {
                    TextField("App ID（cli_xxx）", text: $appId)
                        .textFieldStyle(.roundedBorder)
                    SecureField("App Secret", text: $appSecret)
                        .textFieldStyle(.roundedBorder)
                    TextField("单聊/群 Chat ID（oc_xxx）", text: $chatId)
                        .textFieldStyle(.roundedBorder)
                    Text("获取方式：飞书里和机器人单聊 → 右上角 ⋯ → 群设置 → 查看群 ID（单聊/群聊都支持）")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                } else {
                    TextField("Webhook 地址", text: $webhook)
                        .textFieldStyle(.roundedBorder)
                    SecureField("安全密钥（可选）", text: $webhookSecret)
                        .textFieldStyle(.roundedBorder)
                }

                HStack(spacing: 8) {
                    if saved {
                        Text("已保存 ✓").font(.caption).foregroundColor(.green)
                    } else {
                        Text(controller.feishuConfigured ? "已配置 ✅" : "未配置")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                    if mode == "app" {
                        Button("查群列表") { controller.runFeishuCli(["feishu", "chats"]) }
                    }
                    Button("测试发送") { controller.runFeishuCli(["feishu", "test"]) }
                    Button("保存") { saveFeishu() }
                }

                if !controller.feishuResult.isEmpty {
                    Text(controller.feishuResult)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            Divider()

            Group {
                Text("最近事件").font(.headline)
                if controller.recentEvents.isEmpty {
                    Text("（暂无）").font(.caption).foregroundColor(.secondary)
                }
                ForEach(controller.recentEvents, id: \.self) { Text($0).font(.caption) }
            }

            Divider()

            HStack {
                Button("打开数据目录") { controller.openStateDir() }
                Spacer()
                Button("退出") { NSApp.terminate(nil) }
            }
        }
        .padding(14)
        .frame(width: 380)
        .onAppear {
            let cfg = AppConfigFile.load()
            if !(cfg["feishuAppId"] ?? "").isEmpty { mode = "app" }
            else if !(cfg["feishuWebhook"] ?? "").isEmpty { mode = "webhook" }
            appId = cfg["feishuAppId"] ?? ""
            appSecret = cfg["feishuAppSecret"] ?? ""
            chatId = cfg["feishuChatId"] ?? ""
            webhook = cfg["feishuWebhook"] ?? ""
            webhookSecret = cfg["feishuSecret"] ?? ""
            controller.refresh()
        }
    }

    private func saveFeishu() {
        var cfg = AppConfigFile.load()
        cfg["feishuAppId"] = appId.trimmingCharacters(in: .whitespacesAndNewlines)
        cfg["feishuAppSecret"] = appSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        cfg["feishuChatId"] = chatId.trimmingCharacters(in: .whitespacesAndNewlines)
        cfg["feishuWebhook"] = webhook.trimmingCharacters(in: .whitespacesAndNewlines)
        cfg["feishuSecret"] = webhookSecret.trimmingCharacters(in: .whitespacesAndNewlines)
        AppConfigFile.save(cfg)
        saved = true
        controller.refresh()
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { saved = false }
    }
}
