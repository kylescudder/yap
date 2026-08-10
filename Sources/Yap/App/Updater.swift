import AppKit
import Sparkle

/// Wraps Sparkle's standard updater. Reads SUFeedURL + SUPublicEDKey from Info.plist.
/// The "Check for Updates…" menu item targets `controller` directly (Sparkle handles validation).
final class Updater {
    let controller: SPUStandardUpdaterController

    init() {
        controller = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: nil,
            userDriverDelegate: nil
        )
    }
}
