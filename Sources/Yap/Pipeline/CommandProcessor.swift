import Foundation
import FoundationModels

/// Command Mode: transform selected text (or generate at cursor) per a spoken instruction.
/// Uses the on-device Foundation Model with an instruction-FOLLOWING prompt — the inverse of
/// the cleanup path, which never obeys the transcript.
enum CommandProcessor {
    static func process(instruction: String, selection: String?) async throws -> String {
        if #available(macOS 26.0, *) {
            return try await FoundationCommand.run(instruction: instruction, selection: selection)
        }
        throw NSError(domain: "Yap.Command", code: 1,
                      userInfo: [NSLocalizedDescriptionKey: "Command Mode requires macOS 26."])
    }
}

@available(macOS 26.0, *)
private enum FoundationCommand {
    static func run(instruction: String, selection: String?) async throws -> String {
        let hasSelection = !(selection ?? "").trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        let session = LanguageModelSession(instructions: instructions(hasSelection: hasSelection))
        let options = GenerationOptions(temperature: 0.3, maximumResponseTokens: 1024)
        return try await session
            .respond(to: prompt(instruction: instruction, selection: selection, hasSelection: hasSelection),
                     options: options)
            .content
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func instructions(hasSelection: Bool) -> String {
        hasSelection
            ? "You are a precise text-editing assistant. Apply the user's instruction to the provided text and output ONLY the edited text — no preamble, quotes, or commentary."
            : "You are a concise writing assistant. Follow the user's instruction and output ONLY the requested text — no preamble, quotes, or commentary."
    }

    private static func prompt(instruction: String, selection: String?, hasSelection: Bool) -> String {
        if hasSelection, let selection {
            return """
            Instruction: \(instruction)

            Apply the instruction to the text between the markers and return only the edited version:
            ⟦TEXT START⟧
            \(selection)
            ⟦TEXT END⟧
            """
        }
        return instruction
    }
}
