import Foundation
import WhisperKit

/// Stage-1 transcription via WhisperKit (on-device CoreML Whisper). The model is downloaded
/// once from Hugging Face into Application Support and cached. An actor serializes access so
/// the model loads exactly once even under concurrent prepare/transcribe calls.
actor WhisperKitTranscriber: Transcriber {
    /// Turbo variant: near-large-v3 accuracy, several× faster. Matches the model directory in
    /// the argmaxinc/whisperkit-coreml repo.
    private let modelName = "large-v3-v20240930_turbo"

    private var whisper: WhisperKit?
    private var loadTask: Task<WhisperKit, Error>?

    /// Warm up the model ahead of first use (downloads if needed).
    func prepare() async {
        _ = try? await loadIfNeeded()
    }

    func transcribe(_ audio: [Float], sampleRate: Double) async throws -> Transcript {
        let pipe = try await loadIfNeeded()
        let results = try await pipe.transcribe(audioArray: audio)
        let text = results
            .map { $0.text }
            .joined(separator: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return Transcript(text: text, detectedLanguage: results.first?.language)
    }

    private func loadIfNeeded() async throws -> WhisperKit {
        if let whisper { return whisper }
        if let loadTask { return try await loadTask.value }

        let name = modelName
        let task = Task { () throws -> WhisperKit in
            do {
                Log.info("Loading WhisperKit model \(name)…")
                // prewarm runs the one-time ANE model specialization during load (at launch,
                // in the background) instead of blocking the user's first dictation.
                return try await WhisperKit(WhisperKitConfig(model: name, prewarm: true))
            } catch {
                Log.error("WhisperKit '\(name)' failed (\(error)); falling back to default model.")
                return try await WhisperKit(WhisperKitConfig(prewarm: true))
            }
        }
        loadTask = task
        let pipe = try await task.value
        whisper = pipe
        Log.info("WhisperKit ready.")
        return pipe
    }
}
