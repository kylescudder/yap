import SwiftUI
import Combine

struct SettingsView: View {
    @EnvironmentObject var settings: AppSettings
    @EnvironmentObject var permissions: PermissionsManager

    private let ticker = Timer.publish(every: 1.0, on: .main, in: .common).autoconnect()

    private var audioModeHint: String {
        switch settings.audioMode {
        case .off:
            return "Yap won't change your audio."
        case .lower:
            return "Yap dips the system volume while you speak, then restores it. Only when audio is actually playing."
        case .pause:
            return "Yap pauses whatever's playing when you start, and resumes it when you release. Only when audio is actually playing."
        }
    }

    var body: some View {
        Form {
            Section("Push-to-Talk") {
                RecorderField(title: "Hold key", binding: $settings.pushToTalkKey)
                if settings.pushToTalkKey.interceptsTyping {
                    Text("This key has no modifiers, so Yap will intercept it everywhere (you won't be able to type it).")
                        .font(.caption).foregroundStyle(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Text("Hold to dictate; double-tap to lock hands-free.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Command Mode") {
                RecorderField(title: "Hold key", binding: $settings.commandKey)
                if settings.commandKey == settings.pushToTalkKey {
                    Text("Must differ from the push-to-talk key.")
                        .font(.caption).foregroundStyle(.orange)
                } else if settings.commandKey.interceptsTyping {
                    Text("This key has no modifiers, so Yap will intercept it everywhere.")
                        .font(.caption).foregroundStyle(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Text("Hold and speak an instruction. Selected text is rewritten; otherwise text is generated at the cursor.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Section("Music & Audio") {
                Picker("While dictating", selection: $settings.audioMode) {
                    ForEach(DictationAudioMode.allCases) { Text($0.displayName).tag($0) }
                }
                if settings.audioMode == .lower {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            Text("Level while speaking")
                            Spacer()
                            Text(settings.duckLevel <= 0.001 ? "Mute" : "\(Int((settings.duckLevel * 100).rounded()))%")
                                .foregroundStyle(.secondary)
                                .monospacedDigit()
                        }
                        Slider(value: $settings.duckLevel, in: 0...1, step: 0.05) {
                            Text("Level")
                        } minimumValueLabel: {
                            Text("Mute").font(.caption2).foregroundStyle(.secondary)
                        } maximumValueLabel: {
                            Text("Full").font(.caption2).foregroundStyle(.secondary)
                        }
                    }
                }
                Text(audioModeHint)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Section("Cleanup") {
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Text("Intensity")
                        Spacer()
                        Text(settings.cleanupIntensity.displayName)
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                    Slider(
                        value: Binding(
                            get: { Double(CleanupIntensity.allCases.firstIndex(of: settings.cleanupIntensity) ?? 2) },
                            set: { settings.cleanupIntensity = CleanupIntensity.allCases[Int($0.rounded())] }
                        ),
                        in: 0...Double(CleanupIntensity.allCases.count - 1),
                        step: 1
                    ) {
                        Text("Intensity")
                    } minimumValueLabel: {
                        Text("Off").font(.caption2).foregroundStyle(.secondary)
                    } maximumValueLabel: {
                        Text("Max").font(.caption2).foregroundStyle(.secondary)
                    }
                    Text("Off inserts the raw transcript. Higher levels remove more filler and tidy structure more aggressively — all on-device.")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Divider()
                Toggle("Trim courtesy words (please, thank you)", isOn: $settings.trimCourtesy)
                Text("Drops “please”/“thank you” at the start or end of a sentence, or as a sentence on their own — but keeps them mid-sentence (e.g. “could you please review”).")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Section("Permissions") {
                ForEach(PermissionsManager.required) { PermissionRow(kind: $0) }
                if permissions.allRequiredGranted {
                    Label("All permissions granted", systemImage: "checkmark.seal.fill")
                        .foregroundStyle(.green)
                        .font(.caption)
                }
            }
        }
        .formStyle(.grouped)
        .frame(width: 480, height: 560)
        .onAppear { permissions.refresh() }
        .onReceive(ticker) { _ in permissions.refresh() }
    }
}
