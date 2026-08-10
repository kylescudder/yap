import AppKit

/// A scrolling bar waveform driven by recent audio RMS levels. In `rainbow` mode (used when
/// cleanup intensity is Max) the bars shimmer through the hue spectrum with a moving phase.
final class WaveformView: NSView {
    private var levels: [CGFloat] = []
    private let maxBars = 30
    private let barWidth: CGFloat = 3
    private let gap: CGFloat = 2

    private var rainbow = false
    private var phase: CGFloat = 0
    private var timer: Timer?

    /// Begin a session; clears history and starts the shimmer animation if rainbow is on.
    func start(rainbow: Bool) {
        self.rainbow = rainbow
        levels.removeAll()
        needsDisplay = true
        timer?.invalidate()
        timer = nil
        guard rainbow else { return }
        let t = Timer(timeInterval: 1.0 / 30.0, repeats: true) { [weak self] _ in
            guard let self else { return }
            self.phase += 0.015
            self.needsDisplay = true
        }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    func push(_ level: CGFloat) {
        levels.append(min(1, max(0.05, level)))
        if levels.count > maxBars { levels.removeFirst(levels.count - maxBars) }
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        guard !levels.isEmpty else { return }
        let count = min(levels.count, maxBars)
        let totalW = CGFloat(count) * barWidth + CGFloat(max(0, count - 1)) * gap
        var x = (bounds.width - totalW) / 2
        let slice = Array(levels.suffix(count))

        for (i, lvl) in slice.enumerated() {
            let color: NSColor
            if rainbow {
                let hue = (CGFloat(i) / CGFloat(max(1, count)) + phase).truncatingRemainder(dividingBy: 1.0)
                color = NSColor(hue: hue, saturation: 0.85, brightness: 1.0, alpha: 0.95)
            } else {
                color = NSColor.systemGreen.withAlphaComponent(0.9)
            }
            color.setFill()
            let h = max(2, lvl * bounds.height)
            let y = (bounds.height - h) / 2
            NSBezierPath(
                roundedRect: NSRect(x: x, y: y, width: barWidth, height: h),
                xRadius: barWidth / 2, yRadius: barWidth / 2
            ).fill()
            x += barWidth + gap
        }
    }

    deinit { timer?.invalidate() }
}
