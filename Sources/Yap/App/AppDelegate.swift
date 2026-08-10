import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var menuBar: MenuBarController?
    private var dictation: DictationController?
    private var updater: Updater?
    let permissions = PermissionsManager()
    let settings = AppSettings.shared

    func applicationDidFinishLaunching(_ notification: Notification) {
        let controller = DictationController(settings: settings, permissions: permissions)
        dictation = controller
        let updater = Updater()
        self.updater = updater
        menuBar = MenuBarController(permissions: permissions, settings: settings, dictation: controller, updater: updater)

        // First-run / missing-permission: walk the user through onboarding.
        permissions.refresh()
        if !permissions.allRequiredGranted {
            OnboardingWindowController.shared.show(permissions: permissions, settings: settings)
        }

        // Arm push-to-talk (needs Input Monitoring; user can re-arm from the menu after granting).
        controller.start()

        Log.info("Yap launched. Push-to-talk: \(settings.pushToTalkKey.display).")
    }
}
