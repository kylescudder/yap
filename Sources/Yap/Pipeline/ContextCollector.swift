import AppKit

/// Gathers lightweight context about where dictated text will land, to steer cleanup.
/// Reads only the frontmost app's identity — no field contents — so it needs no extra
/// permission and never touches secure fields. Call on the main thread.
enum ContextCollector {
    static func collect() -> FieldContext {
        let app = NSWorkspace.shared.frontmostApplication
        let bundleID = app?.bundleIdentifier
        let name = app?.localizedName
        return FieldContext(
            appBundleID: bundleID,
            appName: name,
            category: PerAppRules.category(bundleID: bundleID, name: name),
            url: nil
        )
    }
}
