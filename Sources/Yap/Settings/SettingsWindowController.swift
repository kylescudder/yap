import AppKit
import SwiftUI

/// Hosts `SettingsView` in a standard AppKit window (SwiftUI content via NSHostingController).
final class SettingsWindowController {
    static let shared = SettingsWindowController()
    private var window: NSWindow?

    func show(settings: AppSettings, permissions: PermissionsManager) {
        if window == nil {
            let root = SettingsView()
                .environmentObject(settings)
                .environmentObject(permissions)
            let hosting = NSHostingController(rootView: root)
            let w = NSWindow(contentViewController: hosting)
            w.title = "Yap Settings"
            w.styleMask = [.titled, .closable, .miniaturizable]
            w.setContentSize(NSSize(width: 480, height: 340))
            w.isReleasedWhenClosed = false
            window = w
        }
        NSApp.activate()
        window?.center()
        window?.makeKeyAndOrderFront(nil)
    }
}
