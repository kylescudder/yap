import Foundation
import Combine

enum CleanupIntensity: String, CaseIterable, Identifiable {
    case off, light, medium, high, max
    var id: String { rawValue }
    var displayName: String {
        switch self {
        case .off:    return "Off"
        case .light:  return "Light"
        case .medium: return "Medium"
        case .high:   return "High"
        case .max:    return "Max"
        }
    }
}

/// UserDefaults-backed preferences. Named `AppSettings` (not `Settings`) to avoid clashing
/// with SwiftUI's `Settings` scene type.
final class AppSettings: ObservableObject {
    static let shared = AppSettings()

    private let defaults = UserDefaults.standard

    @Published var cleanupIntensity: CleanupIntensity {
        didSet { defaults.set(cleanupIntensity.rawValue, forKey: Keys.intensity) }
    }
    @Published var pushToTalkKey: KeyBinding {
        didSet { saveBinding(pushToTalkKey, Keys.pushToTalkKey) }
    }
    @Published var commandKey: KeyBinding {
        didSet { saveBinding(commandKey, Keys.commandKey) }
    }

    private enum Keys {
        static let intensity = "cleanupIntensity"
        static let pushToTalkKey = "pushToTalkKeyBinding"
        static let commandKey = "commandKeyBinding"
    }

    private init() {
        cleanupIntensity = CleanupIntensity(rawValue: defaults.string(forKey: Keys.intensity) ?? "") ?? .medium
        pushToTalkKey = AppSettings.loadBinding(defaults, Keys.pushToTalkKey) ?? .pushToTalkDefault
        commandKey = AppSettings.loadBinding(defaults, Keys.commandKey) ?? .commandDefault
    }

    private func saveBinding(_ binding: KeyBinding, _ key: String) {
        if let data = try? JSONEncoder().encode(binding) { defaults.set(data, forKey: key) }
    }

    private static func loadBinding(_ defaults: UserDefaults, _ key: String) -> KeyBinding? {
        guard let data = defaults.data(forKey: key) else { return nil }
        return try? JSONDecoder().decode(KeyBinding.self, from: data)
    }
}
