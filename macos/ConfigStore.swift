import Foundation

// MARK: - App config file helpers (~/.ai-task-notify/config.json)

/// Data layer: read/write config.json, token, events.jsonl.
/// Owns no process management and no UI.
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

    static func save(_ cfg: [String: String]) {
        ensureDir()
        let data = try! JSONSerialization.data(withJSONObject: cfg, options: [.prettyPrinted, .sortedKeys])
        try? data.write(to: configFile, options: .atomic)
    }

    static func readToken() -> String {
        ensureDir()
        return (try? String(contentsOf: tokenFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)) ?? ""
    }

    /// Latest events rendered as short human-readable lines (newest first).
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
