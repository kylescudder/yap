import AppKit
import CoreGraphics

/// Reads the current selection from the frontmost app by copying it (⌘C) and reading the
/// pasteboard, then restoring the prior clipboard. Async because the target app writes to the
/// pasteboard on its own runloop.
enum SelectionReader {
    static func readSelection(completion: @escaping (String?) -> Void) {
        let pasteboard = NSPasteboard.general
        let previous = pasteboard.string(forType: .string)
        let before = pasteboard.changeCount

        sendCopy()

        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) {
            let selection = pasteboard.changeCount != before ? pasteboard.string(forType: .string) : nil
            pasteboard.clearContents()
            if let previous { pasteboard.setString(previous, forType: .string) }
            completion(selection)
        }
    }

    private static func sendCopy() {
        let source = CGEventSource(stateID: .combinedSessionState)
        let key: CGKeyCode = 8 // ANSI 'c'
        let down = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: true)
        let up = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: false)
        down?.flags = .maskCommand
        up?.flags = .maskCommand
        down?.post(tap: .cgAnnotatedSessionEventTap)
        up?.post(tap: .cgAnnotatedSessionEventTap)
    }
}
