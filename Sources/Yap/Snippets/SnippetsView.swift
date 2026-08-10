import SwiftUI

struct SnippetsView: View {
    @EnvironmentObject var store: SnippetStore
    @State private var trigger = ""
    @State private var expansion = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Snippets").font(.headline)
                Spacer()
            }
            .padding(12)
            Divider()

            Group {
                if store.snippets.isEmpty {
                    VStack {
                        Spacer()
                        Text("No snippets yet. Add one below — say the trigger and Yap expands it.")
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 24)
                        Spacer()
                    }
                } else {
                    List {
                        ForEach(store.snippets) { snippet in
                            HStack(alignment: .top, spacing: 10) {
                                Text(snippet.trigger).bold().frame(width: 120, alignment: .leading)
                                Text(snippet.expansion).foregroundStyle(.secondary)
                                    .fixedSize(horizontal: false, vertical: true)
                                Spacer()
                                Button(role: .destructive) { store.remove(snippet.id) } label: {
                                    Image(systemName: "trash")
                                }
                                .buttonStyle(.borderless)
                            }
                            .padding(.vertical, 3)
                        }
                    }
                    .listStyle(.inset)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            Divider()
            VStack(alignment: .leading, spacing: 8) {
                Text("Add snippet").font(.subheadline).bold()
                TextField("Trigger (e.g. my email)", text: $trigger)
                TextField("Expands to…", text: $expansion, axis: .vertical)
                    .lineLimit(1...4)
                HStack {
                    Spacer()
                    Button("Add") {
                        store.add(trigger: trigger, expansion: expansion)
                        trigger = ""; expansion = ""
                    }
                    .keyboardShortcut(.defaultAction)
                    .disabled(trigger.trimmingCharacters(in: .whitespaces).isEmpty
                              || expansion.trimmingCharacters(in: .whitespaces).isEmpty)
                }
            }
            .textFieldStyle(.roundedBorder)
            .padding(12)
        }
        .frame(width: 460, height: 460)
    }
}
