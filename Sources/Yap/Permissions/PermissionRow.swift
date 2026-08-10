import SwiftUI

/// A single permission row with live status and a grant/open-settings action.
/// Shared by first-run onboarding and the Settings window.
struct PermissionRow: View {
    @EnvironmentObject var permissions: PermissionsManager
    let kind: PermissionKind

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            statusIcon
                .font(.title3)
                .frame(width: 22)

            VStack(alignment: .leading, spacing: 2) {
                Text(kind.title).font(.headline)
                Text(kind.why)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Spacer()
            actionButton
        }
        .padding(.vertical, 4)
    }

    private var status: PermissionStatus { permissions.status(kind) }

    @ViewBuilder private var statusIcon: some View {
        switch status {
        case .granted:       Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
        case .denied:        Image(systemName: "exclamationmark.circle.fill").foregroundStyle(.orange)
        case .notDetermined: Image(systemName: "circle.dashed").foregroundStyle(.secondary)
        }
    }

    @ViewBuilder private var actionButton: some View {
        switch status {
        case .granted:
            Text("Granted").font(.caption).foregroundStyle(.green)
        case .notDetermined where kind == .microphone:
            Button("Grant") { permissions.request(kind) }
                .buttonStyle(.borderedProminent)
        default:
            Button("Open Settings") { permissions.request(kind) }
                .buttonStyle(.bordered)
        }
    }
}
