import AppKit

/// A scrolling bar waveform driven by recent audio RMS levels. Off-white bars to match the
/// Graphite logo (no rainbow — that treatment was dropped).
final class WaveformView: NSView {
    private var levels: [CGFloat] = []
    private let maxBars = 30
    private let barWidth: CGFloat = 3
    private let gap: CGFloat = 2

    /// Brand off-white, matching the logo's waveform.
    var barColor = NSColor(srgbRed: 0xEC / 255, green: 0xEC / 255, blue: 0xF0 / 255, alpha: 0.95)

    func start() {
        levels.removeAll()
        needsDisplay = true
    }

    func stop() { /* nothing to tear down */ }

    func push(_ level: CGFloat) {
        levels.append(min(1, max(0.05, level)))
        if levels.count > maxBars { levels.removeFirst(levels.count - maxBars) }
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        guard !levels.isEmpty else { return }
        barColor.setFill()

        let count = min(levels.count, maxBars)
        let totalW = CGFloat(count) * barWidth + CGFloat(max(0, count - 1)) * gap
        var x = (bounds.width - totalW) / 2
        for lvl in levels.suffix(count) {
            let h = max(2, lvl * bounds.height)
            let y = (bounds.height - h) / 2
            NSBezierPath(
                roundedRect: NSRect(x: x, y: y, width: barWidth, height: h),
                xRadius: barWidth / 2, yRadius: barWidth / 2
            ).fill()
            x += barWidth + gap
        }
    }
}
