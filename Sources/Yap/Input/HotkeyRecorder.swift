import AppKit
import CoreGraphics

/// Captures the next key/chord (or a bare modifier press-release) via a local event monitor while
/// the Settings window is focused, and returns it as a `KeyBinding`. Esc cancels.
final class HotkeyRecorder {
    var onCapture: ((KeyBinding?) -> Void)?

    private var monitor: Any?
    private var pendingModifier: (keyCode: UInt16, mask: CGEventFlags, display: String)?

    func start() {
        stop()
        pendingModifier = nil
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .flagsChanged]) { [weak self] event in
            self?.handle(event)
            return nil // swallow so it doesn't act on the Settings window
        }
    }

    func stop() {
        if let monitor { NSEvent.removeMonitor(monitor); self.monitor = nil }
    }

    private func handle(_ event: NSEvent) {
        switch event.type {
        case .keyDown:
            if event.keyCode == 53 { // Esc cancels
                finish(nil); return
            }
            let mods = Self.cgMask(from: event.modifierFlags)
            let binding = KeyBinding(
                keyCode: Int(event.keyCode),
                modifiers: mods.rawValue,
                isModifierOnly: false,
                display: Self.modifierSymbols(mods) + Self.keyLabel(event))
            finish(binding)

        case .flagsChanged:
            guard let info = Self.modifierInfo(keyCode: event.keyCode) else { return }
            if event.modifierFlags.contains(info.flag) {
                pendingModifier = (event.keyCode, info.mask, info.display)
            } else if let pending = pendingModifier, pending.keyCode == event.keyCode {
                let binding = KeyBinding(
                    keyCode: Int(pending.keyCode),
                    modifiers: pending.mask.rawValue,
                    isModifierOnly: true,
                    display: pending.display)
                finish(binding)
            }

        default:
            break
        }
    }

    private func finish(_ binding: KeyBinding?) {
        stop()
        onCapture?(binding)
    }

    // MARK: - Mapping helpers

    private static func cgMask(from ns: NSEvent.ModifierFlags) -> CGEventFlags {
        var f: CGEventFlags = []
        if ns.contains(.command) { f.insert(.maskCommand) }
        if ns.contains(.shift) { f.insert(.maskShift) }
        if ns.contains(.option) { f.insert(.maskAlternate) }
        if ns.contains(.control) { f.insert(.maskControl) }
        return f
    }

    private static func modifierSymbols(_ f: CGEventFlags) -> String {
        var s = ""
        if f.contains(.maskControl) { s += "⌃" }
        if f.contains(.maskAlternate) { s += "⌥" }
        if f.contains(.maskShift) { s += "⇧" }
        if f.contains(.maskCommand) { s += "⌘" }
        return s
    }

    private static func modifierInfo(keyCode: UInt16) -> (flag: NSEvent.ModifierFlags, mask: CGEventFlags, display: String)? {
        switch keyCode {
        case 56: return (.shift, .maskShift, "Left ⇧")
        case 60: return (.shift, .maskShift, "Right ⇧")
        case 59: return (.control, .maskControl, "Left ⌃")
        case 62: return (.control, .maskControl, "Right ⌃")
        case 58: return (.option, .maskAlternate, "Left ⌥")
        case 61: return (.option, .maskAlternate, "Right ⌥")
        case 55: return (.command, .maskCommand, "Left ⌘")
        case 54: return (.command, .maskCommand, "Right ⌘")
        case 63: return (.function, .maskSecondaryFn, "Fn (Globe)")
        default: return nil
        }
    }

    private static let specialKeys: [UInt16: String] = [
        49: "Space", 36: "Return", 48: "Tab", 53: "Esc", 51: "Delete",
        123: "←", 124: "→", 125: "↓", 126: "↑",
        122: "F1", 120: "F2", 99: "F3", 118: "F4", 96: "F5", 97: "F6",
        98: "F7", 100: "F8", 101: "F9", 109: "F10", 103: "F11", 111: "F12",
    ]

    private static func keyLabel(_ event: NSEvent) -> String {
        if let s = specialKeys[event.keyCode] { return s }
        if let ch = event.charactersIgnoringModifiers, !ch.isEmpty, ch != " " {
            return ch.uppercased()
        }
        return "Key\(event.keyCode)"
    }
}
