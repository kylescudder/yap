import SwiftUI
import Combine
import AppKit

struct OnboardingView: View {
    @EnvironmentObject var permissions: PermissionsManager
    @EnvironmentObject var settings: AppSettings
    var onDone: () -> Void = {}

    // Poll so toggles flipped in System Settings reflect here without a manual refresh.
    private let ticker = Timer.publish(every: 1.0, on: .main, in: .common).autoconnect()

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            header

            if permissions.allRequiredGranted {
                readyCard
            } else {
                VStack(spacing: 6) {
                    ForEach(PermissionsManager.required) { PermissionRow(kind: $0) }
                }
                stuckHelp
            }

            Spacer(minLength: 0)
            footer
        }
        .padding(28)
        .frame(width: 540, height: 520)
        .onReceive(ticker) { _ in permissions.refresh() }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Welcome to Yap").font(.largeTitle.bold())
            Text("Local-first voice dictation. Everything runs on your Mac — your voice never leaves the machine.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var readyCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("You're all set", systemImage: "checkmark.seal.fill")
                .font(.title2.bold())
                .foregroundStyle(.green)
            Text("Hold **\(settings.pushToTalkKey.display)** in any text field, speak, then release. "
                 + "Cleaned text is inserted at your cursor.")
                .fixedSize(horizontal: false, vertical: true)
            Text("Change the key or engines anytime from the menu-bar icon → Settings.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(18)
        .background(.green.opacity(0.12), in: RoundedRectangle(cornerRadius: 14))
    }

    private var stuckHelp: some View {
        DisclosureGroup("Toggled it on but still showing red?") {
            VStack(alignment: .leading, spacing: 8) {
                Text("An older build of Yap may be listed. Remove any existing “Yap” entry with the "
                     + "“–” button in System Settings, then add this exact app:")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                Button {
                    NSWorkspace.shared.activateFileViewerSelecting([Bundle.main.bundleURL])
                } label: {
                    Label("Reveal Yap in Finder", systemImage: "folder")
                }
            }
            .padding(.top, 6)
        }
        .font(.callout)
    }

    private var footer: some View {
        HStack {
            if permissions.allRequiredGranted {
                Button("Done", action: onDone)
                    .keyboardShortcut(.defaultAction)
            } else {
                Button("Refresh") { permissions.refresh() }
            }
            Spacer()
            Text("Menu bar → Check Permissions… reopens this window.")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
    }
}

