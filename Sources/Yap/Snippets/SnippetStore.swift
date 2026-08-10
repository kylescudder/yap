import Foundation
import Combine

struct Snippet: Identifiable, Codable {
    var id = UUID()
    var trigger: String
    var expansion: String
}

/// Local store of text snippets: a spoken/typed trigger phrase expands to a longer block.
/// Applied to the cleaned transcript before insertion.
final class SnippetStore: ObservableObject {
    static let shared = SnippetStore()

    @Published private(set) var snippets: [Snippet] = []

    private let fileURL: URL

    private init() {
        let dir = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Yap", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        fileURL = dir.appendingPathComponent("snippets.json")
        load()
    }

    func add(trigger: String, expansion: String) {
        let t = trigger.trimmingCharacters(in: .whitespacesAndNewlines)
        let e = expansion.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, !e.isEmpty else { return }
        snippets.append(Snippet(trigger: t, expansion: e))
        save()
    }

    func remove(_ id: UUID) {
        snippets.removeAll { $0.id == id }
        save()
    }

    /// Replace whole-word, case-insensitive occurrences of each trigger with its expansion.
    func apply(to text: String) -> String {
        var result = text
        for snippet in snippets where !snippet.trigger.isEmpty {
            let pattern = "\\b" + NSRegularExpression.escapedPattern(for: snippet.trigger) + "\\b"
            guard let re = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else { continue }
            let range = NSRange(result.startIndex..., in: result)
            result = re.stringByReplacingMatches(
                in: result, options: [], range: range,
                withTemplate: NSRegularExpression.escapedTemplate(for: snippet.expansion)
            )
        }
        return result
    }

    private func load() {
        guard let data = try? Data(contentsOf: fileURL),
              let decoded = try? JSONDecoder().decode([Snippet].self, from: data) else { return }
        snippets = decoded
    }

    private func save() {
        guard let data = try? JSONEncoder().encode(snippets) else { return }
        try? data.write(to: fileURL, options: .atomic)
    }
}
