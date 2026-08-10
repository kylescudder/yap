import Foundation

/// Coarse category of the app being dictated into, used to steer cleanup tone/format.
enum AppCategory: String {
    case email, chat, code, notes, other

    /// Tone guidance injected into the cleanup prompt.
    var tone: String {
        switch self {
        case .email: return "professional and appropriately complete, suitable for an email"
        case .chat:  return "casual and concise, suitable for a chat message"
        case .code:  return "precise; preserve code, identifiers, camelCase/snake_case, and technical terms verbatim"
        case .notes: return "clear and neutral"
        case .other: return "clear and neutral"
        }
    }

    var label: String { rawValue }
}

/// Maps the frontmost app (bundle id + name) to a writing category. Substring match against
/// both, so e.g. "Cursor" (odd bundle id) is caught by name.
enum PerAppRules {
    static func category(bundleID: String?, name: String?) -> AppCategory {
        let hay = [bundleID ?? "", name ?? ""].joined(separator: " ").lowercased()
        guard !hay.trimmingCharacters(in: .whitespaces).isEmpty else { return .other }
        if email.contains(where: hay.contains) { return .email }
        if chat.contains(where: hay.contains)  { return .chat }
        if code.contains(where: hay.contains)  { return .code }
        if notes.contains(where: hay.contains) { return .notes }
        return .other
    }

    private static let email = ["com.apple.mail", "outlook", "spark", "airmail", "superhuman", "proton mail"]
    private static let chat  = ["slack", "discord", "tinyspeck", "whatsapp", "messages", "imessage",
                                "telegram", "teams", "signal", "messenger"]
    private static let code  = ["xcode", "vscode", "visual studio code", "cursor", "windsurf", "iterm",
                                "terminal", "ghostty", "jetbrains", "intellij", "pycharm", "zed", "sublime", "nova"]
    private static let notes = ["notes", "notion", "obsidian", "bear", "craft", "logseq"]
}
