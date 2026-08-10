import AppKit

// Entry point. Run as a menu-bar agent (no Dock icon).
// The bundled .app also sets LSUIElement=true in Info.plist; .accessory here
// makes the raw executable behave the same when run directly.
let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
