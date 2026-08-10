import Foundation

/// Context about where the text will be inserted, fed into the Stage-2 cleanup prompt.
/// Currently the frontmost app identity only (surrounding-text reading is a later increment
/// and will exclude secure/password fields).
struct FieldContext {
    var appBundleID: String? = nil
    var appName: String? = nil
    var category: AppCategory = .other
    var url: String? = nil

    static let empty = FieldContext()
}

/// Stage 2: raw transcript → clean, formatted, context-aware text.
protocol Cleaner {
    /// Optional warm-up before first use. Default: no-op.
    func prepare() async
    func clean(_ transcript: Transcript, context: FieldContext, intensity: CleanupIntensity) async throws -> String
}

extension Cleaner {
    func prepare() async {}
}

/// Fallback cleaner (returns the raw transcript unchanged) — used on macOS versions without
/// the Foundation Models framework.
struct PassthroughCleaner: Cleaner {
    func clean(_ transcript: Transcript, context: FieldContext, intensity: CleanupIntensity) async throws -> String {
        transcript.text
    }
}
