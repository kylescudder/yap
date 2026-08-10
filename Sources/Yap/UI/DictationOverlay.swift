import AppKit

/// A small, compact floating pill shown while dictating — amber dot beside a live waveform on a
/// single line, matching the app icon. Non-activating (never steals focus), no text.
final class DictationOverlay {
    private let panel: NSPanel
    private let dot = NSView()
    private let waveform: WaveformView
    private let spinner = NSProgressIndicator()
    private let label = NSTextField(labelWithString: "")

    private let width: CGFloat = 150
    private let height: CGFloat = 30

    // Brand colours.
    private let amber = NSColor(srgbRed: 1.0, green: 0xB0 / 255, blue: 0x20 / 255, alpha: 1)
    private let neutral = NSColor(srgbRed: 0x8E / 255, green: 0x8E / 255, blue: 0x93 / 255, alpha: 1)
    private let danger = NSColor(srgbRed: 0xFF / 255, green: 0x5A / 255, blue: 0x54 / 255, alpha: 1)

    init() {
        panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                        styleMask: [.borderless, .nonactivatingPanel],
                        backing: .buffered, defer: false)
        panel.level = .statusBar
        panel.isFloatingPanel = true
        panel.hidesOnDeactivate = false
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary, .ignoresCycle, .fullScreenAuxiliary]
        panel.backgroundColor = .clear
        panel.isOpaque = false
        panel.hasShadow = true
        panel.ignoresMouseEvents = true

        let effect = NSVisualEffectView(frame: NSRect(x: 0, y: 0, width: width, height: height))
        effect.material = .hudWindow
        effect.state = .active
        effect.blendingMode = .behindWindow
        effect.wantsLayer = true
        effect.layer?.cornerRadius = height / 2   // full pill
        effect.layer?.masksToBounds = true
        effect.autoresizingMask = [.width, .height]

        let dotSize: CGFloat = 8
        dot.wantsLayer = true
        dot.layer?.cornerRadius = dotSize / 2
        dot.frame = NSRect(x: 14, y: (height - dotSize) / 2, width: dotSize, height: dotSize)
        dot.layer?.backgroundColor = amber.cgColor

        let contentX: CGFloat = 30
        let contentW = width - contentX - 12
        let rowY = (height - 16) / 2

        waveform = WaveformView(frame: NSRect(x: contentX, y: rowY, width: contentW, height: 16))
        waveform.autoresizingMask = [.width]

        spinner.frame = NSRect(x: contentX, y: rowY, width: 16, height: 16)
        spinner.style = .spinning
        spinner.controlSize = .small
        spinner.isDisplayedWhenStopped = false
        spinner.isHidden = true

        label.frame = NSRect(x: contentX, y: rowY, width: contentW, height: 16)
        label.font = .systemFont(ofSize: 11, weight: .medium)
        label.textColor = .labelColor
        label.lineBreakMode = .byTruncatingTail
        label.isHidden = true

        effect.addSubview(dot)
        effect.addSubview(waveform)
        effect.addSubview(spinner)
        effect.addSubview(label)
        panel.contentView = effect
    }

    func showRecording() {
        setDot(amber)
        label.isHidden = true
        spinner.isHidden = true; spinner.stopAnimation(nil)
        waveform.isHidden = false; waveform.start()
        present()
    }

    /// Command Mode looks the same compact pill.
    func showCommand() { showRecording() }

    /// Hands-free (double-tap lock): the pill just stays up — no text.
    func markHandsFree() { setDot(amber) }

    func showProcessing(preparing: Bool = false) {
        setDot(neutral)
        label.isHidden = true
        waveform.isHidden = true; waveform.stop()
        spinner.isHidden = false; spinner.startAnimation(nil)
        present()
    }

    /// Generic working state (Command Mode) — same spinner, no text.
    func showWorking(_ text: String) { showProcessing() }

    /// Brief transient error (rare) — red dot + short message, auto-dismissed.
    func flash(_ message: String) {
        setDot(danger)
        waveform.isHidden = true; waveform.stop()
        spinner.isHidden = true; spinner.stopAnimation(nil)
        label.isHidden = false; label.stringValue = message
        present()
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.4) { [weak self] in self?.hide() }
    }

    func hide() {
        waveform.stop()
        spinner.stopAnimation(nil)
        panel.orderOut(nil)
    }

    func setLevel(_ rms: Float) {
        waveform.push(min(1, max(0, CGFloat(rms) * 6)))
    }

    private func setDot(_ color: NSColor) {
        dot.layer?.backgroundColor = color.cgColor
    }

    private func present() {
        positionBottomCenter()
        panel.orderFrontRegardless()
    }

    private func positionBottomCenter() {
        guard let screen = NSScreen.main else { return }
        let visible = screen.visibleFrame
        panel.setFrameOrigin(NSPoint(x: visible.midX - width / 2, y: visible.minY + 80))
    }
}
