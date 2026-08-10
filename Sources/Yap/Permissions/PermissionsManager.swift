import AppKit
import AVFoundation
import ApplicationServices
import IOKit.hid
import Combine

enum PermissionStatus {
    case granted, denied, notDetermined
}

/// The three system permissions Yap needs on macOS.
enum PermissionKind: String, CaseIterable, Identifiable {
    case microphone
    case accessibility
    case inputMonitoring

    var id: String { rawValue }

    var title: String {
        switch self {
        case .microphone:     return "Microphone"
        case .accessibility:  return "Accessibility"
        case .inputMonitoring: return "Input Monitoring"
        }
    }

    var why: String {
        switch self {
        case .microphone:      return "Capture your voice for on-device transcription."
        case .accessibility:   return "Read the focused text field and insert text into any app."
        case .inputMonitoring: return "Detect the push-to-talk hotkey system-wide."
        }
    }

    /// Deep-link into the relevant System Settings > Privacy pane.
    var settingsURL: URL {
        let base = "x-apple.systempreferences:com.apple.preference.security?"
        switch self {
        case .microphone:      return URL(string: base + "Privacy_Microphone")!
        case .accessibility:   return URL(string: base + "Privacy_Accessibility")!
        case .inputMonitoring: return URL(string: base + "Privacy_ListenEvent")!
        }
    }
}

final class PermissionsManager: ObservableObject {
    @Published private(set) var statuses: [PermissionKind: PermissionStatus] = [:]

    init() { refresh() }

    /// Re-read the live status of every permission.
    func refresh() {
        statuses = [
            .microphone:      microphoneStatus(),
            .accessibility:   AXIsProcessTrusted() ? .granted : .denied,
            .inputMonitoring: inputMonitoringStatus(),
        ]
    }

    /// Only two are needed: Microphone (capture) and Accessibility (which authorizes the
    /// keyboard event tap AND inserting text). Input Monitoring is NOT required.
    static let required: [PermissionKind] = [.microphone, .accessibility]

    var allRequiredGranted: Bool {
        Self.required.allSatisfy { statuses[$0] == .granted }
    }

    func status(_ kind: PermissionKind) -> PermissionStatus {
        statuses[kind] ?? .notDetermined
    }

    /// Trigger the native request (mic) or open the right Settings pane (AX / Input Monitoring).
    func request(_ kind: PermissionKind) {
        switch kind {
        case .microphone:
            AVCaptureDevice.requestAccess(for: .audio) { [weak self] _ in
                DispatchQueue.main.async { self?.refresh() }
            }
        case .accessibility:
            // Fires the system prompt once; also open Settings so the user can flip it.
            let options = ["AXTrustedCheckOptionPrompt": true] as CFDictionary
            _ = AXIsProcessTrustedWithOptions(options)
            openSettings(kind)
        case .inputMonitoring:
            _ = IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)
            openSettings(kind)
        }
    }

    func openSettings(_ kind: PermissionKind) {
        NSWorkspace.shared.open(kind.settingsURL)
    }

    // MARK: - Status probes

    private func microphoneStatus() -> PermissionStatus {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized:            return .granted
        case .denied, .restricted:   return .denied
        case .notDetermined:         return .notDetermined
        @unknown default:            return .notDetermined
        }
    }

    private func inputMonitoringStatus() -> PermissionStatus {
        switch IOHIDCheckAccess(kIOHIDRequestTypeListenEvent) {
        case kIOHIDAccessTypeGranted: return .granted
        case kIOHIDAccessTypeDenied:  return .denied
        default:                      return .notDetermined
        }
    }
}
