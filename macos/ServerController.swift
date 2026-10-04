import AppKit
import Combine
import Foundation

// MARK: - Server controller

/// Process layer: start/stop the bundled Node server, poll state, run the
/// bundled CLI (feishu test/chats). Owns no config-file details and no UI.
final class ServerController: ObservableObject {
    @Published var isRunning = false
    @Published var hostname = ""
    @Published var port = 3210
    @Published var token = ""
    @Published var feishuConfigured = false
    @Published var recentEvents: [String] = []
    @Published var lastError = ""
    @Published var feishuResult = ""

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
        feishuConfigured = !(cfg["feishuAppId"] ?? "").isEmpty || !(cfg["feishuWebhook"] ?? "").isEmpty
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

    /// Run the bundled CLI (feishu test / feishu chats), show the result in the popover.
    func runFeishuCli(_ args: [String]) {
        feishuResult = "正在执行…"
        guard let node = resolveNode(),
              let bundle = Bundle.main.resourceURL?.appendingPathComponent("server/bundle.cjs"),
              FileManager.default.fileExists(atPath: bundle.path) else {
            feishuResult = "❌ 找不到 node 或 server/bundle.cjs"
            return
        }
        let p = Process()
        p.executableURL = URL(fileURLWithPath: node)
        p.arguments = [bundle.path] + args
        var env = ProcessInfo.processInfo.environment
        env["HOME"] = NSHomeDirectory()
        p.environment = env
        let out = Pipe()
        let err = Pipe()
        p.standardOutput = out
        p.standardError = err
        p.terminationHandler = { _ in
            let o = String(data: out.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let e = String(data: err.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let combined = (o + e).trimmingCharacters(in: .whitespacesAndNewlines)
            DispatchQueue.main.async {
                self.feishuResult = combined.isEmpty ? "（无输出）" : combined
            }
        }
        do {
            try p.run()
        } catch {
            feishuResult = "❌ \(error.localizedDescription)"
        }
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
