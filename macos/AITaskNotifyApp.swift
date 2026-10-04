import SwiftUI
import AppKit

// MARK: - App config file helpers (~/.ai-task-notify/config.json)

struct AppConfigFile {
    static let stateDir = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".ai-task-notify")
    static let configFile = stateDir.appendingPathComponent("config.json")
    static let eventsFile = stateDir.appendingPathComponent("events.jsonl")
    static let tokenFile = stateDir.appendingPathComponent("token")
    static let logFile = stateDir.appendingPathComponent("server.log")

    static func ensureDir() {
        try? FileManager.default.createDirectory(at: stateDir, withIntermediateDirectories: true)
    }

    static func load() -> [String: String] {
        ensureDir()
        guard let data = try? Data(contentsOf: configFile),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: String] else { return [:] }
        return obj
    }

    static func save(webhook: String, secret: String) {
        ensureDir()
        var cfg = load()
        cfg["feishuWebhook"] = webhook
        cfg["feishuSecret"] = secret
        let data = try! JSONSerialization.data(withJSONObject: cfg, options: [.prettyPrinted, .sortedKeys])
        try? data.write(to: configFile, options: .atomic)
    }

    static func readToken() -> String {
        ensureDir()
        return (try? String(contentsOf: tokenFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)) ?? ""
    }

    static func readEvents(limit: Int) -> [String] {
        guard let data = try? String(contentsOf: eventsFile, encoding: .utf8) else { return [] }
        let lines = data.split(separator: "\n").suffix(limit)
        var out: [String] = []
        for line in lines {
            guard let obj = try? JSONSerialization.jsonObject(with: Data(line.utf8)) as? [String: Any] else { continue }
            let agent = (obj["agent"] as? String) ?? "?"
            let title = (obj["title"] as? String) ?? ""
            let project = (obj["project"] as? String) ?? ""
            let status = (obj["status"] as? String) ?? ""
            let icon = status == "success" ? "✅" : status == "waiting" ? "🟡" : "💬"
            var lineText = "\(icon) \(agent == "claude" ? "Claude Code" : "Codex") · \(title)"
            if !project.isEmpty { lineText += " · \(project)" }
            out.append(lineText)
        }
        return out.reversed()
    }
}

// MARK: - Server controller

final class ServerController: ObservableObject {
    @Published var isRunning = false
    @Published var hostname = ""
    @Published var port = 3210
    @Published var token = ""
    @Published var feishuConfigured = false
    @Published var recentEvents: [String] = []
    @Published var lastError = ""

    private var process: Process?
    private var timer: Timer?

    func start() {
        guard process == nil else { return }
        guard let node = resolveNode() else {
            lastError = "找不到 Node.js（请确认已安装，或重新构建 App 打包 node）"
            refresh()
            return
        }
        guard let bundle = Bundle.main.resourceURL?.appendingPathComponent("server/bundle.cjs"),
              FileManager.default.fileExists(atPath: bundle.path) else {
            lastError = "App 内缺少 server/bundle.cjs"
            refresh()
            return
        }

        AppConfigFile.ensureDir()
        FileManager.default.createFile(atPath: AppConfigFile.logFile.path, contents: nil)
        let out = FileHandle(forWritingAtPath: AppConfigFile.logFile.path)
        let err = FileHandle(forWritingAtPath: AppConfigFile.logFile.path)

        let p = Process()
        p.executableURL = URL(fileURLWithPath: node)
        p.arguments = [bundle.path, "server", "--port", String(port)]
        var env = ProcessInfo.processInfo.environment
        env["HOME"] = NSHomeDirectory()
        p.environment = env
        p.standardOutput = out
        p.standardError = err
        p.terminationHandler = { [weak self] _ in
            DispatchQueue.main.async {
                self?.process = nil
                self?.refresh()
            }
        }
        do {
            try p.run()
            process = p
            lastError = ""
        } catch {
            lastError = "启动失败：\(error.localizedDescription)"
        }
        startTimer()
        refresh()
    }

    func stop() {
        process?.terminate()
    }

    func refresh() {
        isRunning = (process?.isRunning ?? false)
        hostname = ServerController.localHostname()
        token = AppConfigFile.readToken()
        let cfg = AppConfigFile.load()
        feishuConfigured = !(cfg["feishuWebhook"] ?? "").isEmpty
        recentEvents = AppConfigFile.readEvents(limit: 6)
        if !isRunning, !lastError.isEmpty {
            recentEvents.insert("⚠️ \(lastError)", at: 0)
        }
    }

    func copyConnectURL() {
        let url = "ws://\(hostname):\(port)/ws?token=\(token)"
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(url, forType: .string)
    }

    func copyToken() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(token, forType: .string)
    }

    func openStateDir() {
        NSWorkspace.shared.open(AppConfigFile.stateDir)
    }

    private func startTimer() {
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            DispatchQueue.main.async {
                self?.refresh()
            }
        }
    }

    private static func localHostname() -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/sbin/scutil")
        p.arguments = ["--get", "LocalHostName"]
        let pipe = Pipe()
        p.standardOutput = pipe
        try? p.run()
        p.waitUntilExit()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        let name = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return name.isEmpty ? "unknown.local" : "\(name).local"
    }

    private func resolveNode() -> String? {
        if let bundled = Bundle.main.resourceURL?.appendingPathComponent("node").path,
           FileManager.default.isExecutableFile(atPath: bundled) {
            return bundled
        }
        let nvmd = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".nvmd/versions")
        if let versions = try? FileManager.default.contentsOfDirectory(at: nvmd, includingPropertiesForKeys: nil) {
            let sorted = versions.sorted { $0.lastPathComponent.compare($1.lastPathComponent, options: .numeric) == .orderedDescending }
            for v in sorted {
                let n = v.appendingPathComponent("bin/node").path
                if FileManager.default.isExecutableFile(atPath: n) { return n }
            }
        }
        for path in ["/opt/homebrew/bin/node", "/usr/local/bin/node"] {
            if FileManager.default.isExecutableFile(atPath: path) { return path }
        }
        let which = Process()
        which.executableURL = URL(fileURLWithPath: "/usr/bin/which")
        which.arguments = ["node"]
        let pipe = Pipe()
        which.standardOutput = pipe
        try? which.run()
        which.waitUntilExit()
        let out = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if let out, !out.isEmpty, FileManager.default.isExecutableFile(atPath: out) { return out }
        return nil
    }
}

// MARK: - Menu bar popover content

struct AppMenuView: View {
    @ObservedObject var controller: ServerController
    @State private var webhook: String = ""
    @State private var secret: String = ""
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
                Text("飞书机器人通知").font(.headline)
                TextField("Webhook 地址", text: $webhook)
                    .textFieldStyle(.roundedBorder)
                SecureField("安全密钥（可选）", text: $secret)
                    .textFieldStyle(.roundedBorder)
                HStack(spacing: 8) {
                    if saved {
                        Text("已保存 ✓").font(.caption).foregroundColor(.green)
                    } else {
                        Text(controller.feishuConfigured ? "已配置 ✅" : "未配置")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                    Button("清除") { clearFeishu() }
                    Button("保存") { saveFeishu() }
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
        .frame(width: 360)
        .onAppear {
            let cfg = AppConfigFile.load()
            webhook = cfg["feishuWebhook"] ?? ""
            secret = cfg["feishuSecret"] ?? ""
            controller.refresh()
        }
    }

    private func saveFeishu() {
        AppConfigFile.save(webhook: webhook.trimmingCharacters(in: .whitespacesAndNewlines),
                           secret: secret.trimmingCharacters(in: .whitespacesAndNewlines))
        saved = true
        controller.refresh()
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { saved = false }
    }

    private func clearFeishu() {
        webhook = ""
        secret = ""
        AppConfigFile.save(webhook: "", secret: "")
        saved = true
        controller.refresh()
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { saved = false }
    }
}

// MARK: - App

@main
struct AITaskNotifyApp: App {
    @StateObject private var controller: ServerController

    init() {
        let c = ServerController()
        _controller = StateObject(wrappedValue: c)
        // 启动 App 即自动拉起本地 Server
        c.start()
    }

    var body: some Scene {
        MenuBarExtra {
            AppMenuView(controller: controller)
        } label: {
            Image(systemName: controller.isRunning ? "bell.badge.fill" : "bell.badge")
        }
        .menuBarExtraStyle(.window)
    }
}
