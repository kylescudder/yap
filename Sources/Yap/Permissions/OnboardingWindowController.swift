import AppKit
import SwiftUI

/// Hosts `OnboardingView` — the permission-granting flow shown on first run (and reachable
/// any time via the menu bar's "Check Permissions…"). The window **floats** so it stays
/// visible next to System Settings, and never releases so it can be reopened reliably.
final class OnboardingWindowController {
    static let shared = OnboardingWindowController()
    private var window: NSWindow?

    func show(permissions: PermissionsManager, settings: AppSettings) {
        if window == nil {
            let root = OnboardingView(onDone: { [weak self] in self?.window?.orderOut(nil) })
                .environmentObject(permissions)
                .environmentObject(settings)
            let hosting = NSHostingController(rootView: root)
            let w = NSWindow(contentViewController: hosting)
            w.title = "Welcome to Yap"
            w.styleMask = [.titled, .closable]
            w.level = .floating
            w.isReleasedWhenClosed = false
            w.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
            w.setContentSize(NSSize(width: 540, height: 520))
            window = w
        }
        permissions.refresh()
        NSApp.activate()
        window?.center()
        window?.makeKeyAndOrderFront(nil)
    }
}
