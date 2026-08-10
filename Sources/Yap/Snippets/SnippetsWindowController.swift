import AppKit
import SwiftUI

final class SnippetsWindowController {
    static let shared = SnippetsWindowController()
    private var window: NSWindow?

    func show() {
        if window == nil {
            let root = SnippetsView().environmentObject(SnippetStore.shared)
            let hosting = NSHostingController(rootView: root)
            let w = NSWindow(contentViewController: hosting)
            w.title = "Yap — Snippets"
            w.styleMask = [.titled, .closable, .miniaturizable, .resizable]
            w.setContentSize(NSSize(width: 460, height: 460))
            w.isReleasedWhenClosed = false
            window = w
        }
        NSApp.activate()
        window?.center()
        window?.makeKeyAndOrderFront(nil)
    }
}
