import Foundation
import FoundationModels

/// Stage-2 cleanup via Apple's on-device Foundation Model (macOS 26). Zero dependencies, no
/// download — but requires Apple Intelligence to be enabled, and its safety guardrails may
/// refuse some content. On any failure we return the raw transcript so a dictation is never lost.
///
/// Critical: the transcript is *content to rewrite*, never a prompt to answer. We fence it with
/// explicit markers and repeat that rule, or a dictated question gets answered instead of cleaned.
@available(macOS 26.0, *)
actor FoundationModelsCleaner: Cleaner {
    func clean(_ transcript: Transcript, context: FieldContext, intensity: CleanupIntensity) async throws -> String {
        let raw = transcript.text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard intensity != .off, !raw.isEmpty else { return transcript.text }

        switch SystemLanguageModel.default.availability {
        case .available:
            break
        case .unavailable(let reason):
            Log.error("Apple Foundation model unavailable (\(reason)) — returning raw transcript.")
            return transcript.text
        @unknown default:
            return transcript.text
        }

        do {
            let session = LanguageModelSession(instructions: CleanupPrompt.role)
            let options = GenerationOptions(
                temperature: 0.1,
                maximumResponseTokens: min(2048, max(128, raw.count))
            )
            let cleaned = try await session
                .respond(to: CleanupPrompt.task(for: raw, intensity: intensity, context: context), options: options)
                .content
                .trimmingCharacters(in: .whitespacesAndNewlines)
            return cleaned.isEmpty ? transcript.text : cleaned
        } catch {
            // Guardrail refusal, timeout, etc. — never drop the user's words.
            Log.error("Foundation model cleanup failed (\(error)) — returning raw transcript.")
            return transcript.text
        }
    }
}
