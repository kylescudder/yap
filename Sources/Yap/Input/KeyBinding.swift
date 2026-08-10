import Foundation
import CoreGraphics

/// A recorded hotkey. Either a bare modifier held down (`isModifierOnly`, e.g. Right ⇧) or a
/// regular key / chord (e.g. F5, ⌘⇧Space). For modifier-only, `modifiers` is that modifier's own
/// mask; for regular/chord, `modifiers` is the required modifier mask (0 = none).
struct KeyBinding: Codable, Equatable {
    var keyCode: Int
    var modifiers: UInt64
    var isModifierOnly: Bool
    var display: String

    static let relevantModifiers: CGEventFlags = [.maskCommand, .maskShift, .maskAlternate, .maskControl]

    static let pushToTalkDefault = KeyBinding(
        keyCode: 60, modifiers: CGEventFlags.maskShift.rawValue, isModifierOnly: true, display: "Right ⇧")
    static let commandDefault = KeyBinding(
        keyCode: 61, modifiers: CGEventFlags.maskAlternate.rawValue, isModifierOnly: true, display: "Right ⌥")

    /// True for a bare regular key with no modifiers — will be intercepted globally (can't be typed).
    var interceptsTyping: Bool { !isModifierOnly && modifiers == 0 }
}
