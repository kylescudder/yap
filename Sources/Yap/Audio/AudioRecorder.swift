import AVFoundation

/// Captures microphone audio via `AVAudioEngine`, resampled to mono 16 kHz Float —
/// the format Whisper/Parakeet expect. Push-to-talk batch: `start()` … `stop()` returns
/// the full utterance. Streaming partials come later (Phase 5).
final class AudioRecorder {
    enum RecorderError: Error { case noInput }

    let sampleRate: Double = 16_000
    private let meterBufferSize: AVAudioFrameCount = 512

    /// RMS level callback (0…~1) for the overlay meter, delivered on the main queue.
    var onLevel: ((Float) -> Void)?

    private let engine = AVAudioEngine()
    private var converter: AVAudioConverter?
    private var outFormat: AVAudioFormat?
    private var samples: [Float] = []
    private let lock = NSLock()

    func start() throws {
        lock.lock(); samples.removeAll(keepingCapacity: true); lock.unlock()

        let input = engine.inputNode
        let inputFormat = input.inputFormat(forBus: 0)
        guard inputFormat.sampleRate > 0, inputFormat.channelCount > 0 else {
            throw RecorderError.noInput
        }

        let out = AVAudioFormat(commonFormat: .pcmFormatFloat32,
                                sampleRate: sampleRate,
                                channels: 1,
                                interleaved: false)
        outFormat = out
        converter = out.flatMap { AVAudioConverter(from: inputFormat, to: $0) }

        input.installTap(onBus: 0, bufferSize: meterBufferSize, format: inputFormat) { [weak self] buffer, _ in
            self?.append(buffer)
        }
        engine.prepare()
        try engine.start()
    }

    /// Stops capture and returns the collected 16 kHz mono samples.
    @discardableResult
    func stop() -> [Float] {
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        lock.lock(); let result = samples; lock.unlock()
        return result
    }

    private func append(_ buffer: AVAudioPCMBuffer) {
        guard let converter, let outFormat else { return }

        let ratio = outFormat.sampleRate / buffer.format.sampleRate
        let capacity = AVAudioFrameCount(Double(buffer.frameLength) * ratio) + 1024
        guard let out = AVAudioPCMBuffer(pcmFormat: outFormat, frameCapacity: capacity) else { return }

        var consumed = false
        var convError: NSError?
        converter.convert(to: out, error: &convError) { _, status in
            if consumed { status.pointee = .noDataNow; return nil }
            consumed = true
            status.pointee = .haveData
            return buffer
        }
        guard convError == nil, let channel = out.floatChannelData, out.frameLength > 0 else { return }

        let count = Int(out.frameLength)
        let ptr = UnsafeBufferPointer(start: channel[0], count: count)

        lock.lock(); samples.append(contentsOf: ptr); lock.unlock()

        var sumSquares: Float = 0
        for value in ptr { sumSquares += value * value }
        let rms = (sqrt(sumSquares / Float(count)))
        DispatchQueue.main.async { [weak self] in self?.onLevel?(rms) }
    }
}
