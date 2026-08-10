import SwiftUI
import AppKit

struct HistoryView: View {
    @EnvironmentObject var store: HistoryStore

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Dictation History").font(.headline)
                Spacer()
                Button("Clear", role: .destructive) { store.clear() }
                    .disabled(store.entries.isEmpty)
            }
            .padding(12)
            Divider()

            if store.entries.isEmpty {
                Spacer()
                Text("No dictations yet.").foregroundStyle(.secondary)
                Spacer()
            } else {
                List(store.entries) { entry in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(entry.text)
                            .textSelection(.enabled)
                            .fixedSize(horizontal: false, vertical: true)
                        HStack(spacing: 8) {
                            Text(entry.date, style: .relative).foregroundStyle(.tertiary)
                            if let app = entry.appName {
                                Text("· \(app)").foregroundStyle(.tertiary)
                            }
                            Spacer()
                            Button {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(entry.text, forType: .string)
                            } label: {
                                Image(systemName: "doc.on.doc")
                            }
                            .buttonStyle(.borderless)
                            .help("Copy")
                        }
                        .font(.caption)
                    }
                    .padding(.vertical, 4)
                }
                .listStyle(.inset)
            }
        }
        .frame(width: 460, height: 480)
    }
}
