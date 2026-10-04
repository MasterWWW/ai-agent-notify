import SwiftUI

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
