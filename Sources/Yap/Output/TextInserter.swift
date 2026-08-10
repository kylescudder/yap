import AppKit
import CoreGraphics

/// Inserts text into the frontmost app's focused field.
///
/// Phase 1 strategy: **clipboard → synthetic ⌘V → restore** — the most broadly compatible
/// method (perfect Unicode, fast for long text). AX direct-insertion is added in Phase 4.
/// Requires the **Accessibility** permission to post synthetic key events.
enum TextInserter {
    /// How long to wait before restoring the previous clipboard contents.
    private static let restoreDelay: TimeInterval = 1.2

    static func insert(_ text: String) {
        guard !text.isEmpty else { return }

        let pasteboard = NSPasteboard.general
        let previous = pasteboard.string(forType: .string)

        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)

        postPaste()

        DispatchQueue.main.asyncAfter(deadline: .now() + restoreDelay) {
            pasteboard.clearContents()
            if let previous { pasteboard.setString(previous, forType: .string) }
        }
    }

    /// Posts ⌘V with the modifier flags set *explicitly* to Command only — otherwise a
    /// still-held push-to-talk modifier would corrupt the shortcut.
    private static func postPaste() {
        let source = CGEventSource(stateID: .combinedSessionState)
        let vKey: CGKeyCode = 9 // ANSI 'v'

        let keyDown = CGEvent(keyboardEventSource: source, virtualKey: vKey, keyDown: true)
        let keyUp = CGEvent(keyboardEventSource: source, virtualKey: vKey, keyDown: false)
        keyDown?.flags = .maskCommand
        keyUp?.flags = .maskCommand

        keyDown?.post(tap: .cgAnnotatedSessionEventTap)
        keyUp?.post(tap: .cgAnnotatedSessionEventTap)
    }
}
