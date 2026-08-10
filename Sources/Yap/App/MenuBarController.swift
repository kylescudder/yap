import AppKit
import Sparkle

/// Owns the menu-bar status item and its menu. The status icon will later
/// reflect pipeline state (idle / recording / processing).
final class MenuBarController: NSObject, NSMenuDelegate {
    private let statusItem: NSStatusItem
    private let permissions: PermissionsManager
    private let settings: AppSettings
    private let dictation: DictationController
    private let updater: Updater
    private var hintItem: NSMenuItem?

    init(permissions: PermissionsManager, settings: AppSettings, dictation: DictationController, updater: Updater) {
        self.permissions = permissions
        self.settings = settings
        self.dictation = dictation
        self.updater = updater
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        super.init()

        if let button = statusItem.button {
            button.image = NSImage(systemSymbolName: "mic.fill", accessibilityDescription: "Yap")
            button.image?.isTemplate = true
        }
        buildMenu()
    }

    private func buildMenu() {
        let menu = NSMenu()

        let header = menu.addItem(withTitle: "Yap", action: nil, keyEquivalent: "")
        header.isEnabled = false

        let hint = menu.addItem(withTitle: hintTitle, action: nil, keyEquivalent: "")
        hint.isEnabled = false
        hintItem = hint

        menu.addItem(.separator())

        let restart = menu.addItem(withTitle: "Restart Push-to-Talk", action: #selector(restartHotkey), keyEquivalent: "r")
        restart.target = self

        let historyItem = menu.addItem(withTitle: "Dictation History…", action: #selector(openHistory), keyEquivalent: "h")
        historyItem.target = self

        let snippetsItem = menu.addItem(withTitle: "Snippets…", action: #selector(openSnippets), keyEquivalent: "")
        snippetsItem.target = self

        let settingsItem = menu.addItem(withTitle: "Settings…", action: #selector(openSettings), keyEquivalent: ",")
        settingsItem.target = self

        let updateItem = menu.addItem(withTitle: "Check for Updates…",
                                      action: #selector(SPUStandardUpdaterController.checkForUpdates(_:)),
                                      keyEquivalent: "")
        updateItem.target = updater.controller

        menu.addItem(.separator())

        let quitItem = menu.addItem(withTitle: "Quit Yap", action: #selector(quit), keyEquivalent: "q")
        quitItem.target = self

        menu.delegate = self
        statusItem.menu = menu
    }

    private var hintTitle: String { "Hold \(settings.pushToTalkKey.display) to dictate · double-tap for hands-free" }

    // Keep the hint in sync with the current push-to-talk key each time the menu opens.
    func menuNeedsUpdate(_ menu: NSMenu) {
        hintItem?.title = hintTitle
    }

    @objc private func restartHotkey() {
        dictation.start()
    }

    @objc private func openHistory() {
        HistoryWindowController.shared.show()
    }

    @objc private func openSnippets() {
        SnippetsWindowController.shared.show()
    }

    @objc private func openSettings() {
        permissions.refresh()
        SettingsWindowController.shared.show(settings: settings, permissions: permissions)
    }

    @objc private func quit() {
        NSApplication.shared.terminate(nil)
    }
}
