import Foundation

/// Result of a Stage-1 transcription.
struct Transcript {
    var text: String
    var detectedLanguage: String?
}

/// Stage 1: speech → raw text.
protocol Transcriber {
    /// Optional warm-up (e.g. download/load a model) before first use. Default: no-op.
    func prepare() async
    /// Transcribe a finished utterance (push-to-talk batch). Streaming partials come later.
    func transcribe(_ audio: [Float], sampleRate: Double) async throws -> Transcript
}

extension Transcriber {
    func prepare() async {}
}
