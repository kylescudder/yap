import AppKit

/// A small floating HUD shown near the bottom of the screen while dictating. Non-activating
/// (never steals focus). Palette matches the Graphite logo: amber dot, off-white waveform.
final class DictationOverlay {
    private let panel: NSPanel
    private let dot = NSView()
    private let label = NSTextField(labelWithString: "Listening…")
    private let waveform: WaveformView

    private let width: CGFloat = 240
    private let height: CGFloat = 58

    // Brand colours.
    private let amber = NSColor(srgbRed: 1.0, green: 0xB0 / 255, blue: 0x20 / 255, alpha: 1)
    private let neutral = NSColor(srgbRed: 0x8E / 255, green: 0x8E / 255, blue: 0x93 / 255, alpha: 1)

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
        effect.layer?.cornerRadius = 14
        effect.layer?.masksToBounds = true
        effect.autoresizingMask = [.width, .height]

        dot.wantsLayer = true
        dot.layer?.cornerRadius = 5
        dot.frame = NSRect(x: 16, y: height - 23, width: 10, height: 10)
        dot.layer?.backgroundColor = amber.cgColor

        label.frame = NSRect(x: 34, y: height - 26, width: width - 46, height: 18)
        label.font = .systemFont(ofSize: 13, weight: .medium)
        label.textColor = .labelColor
        label.lineBreakMode = .byTruncatingTail

        waveform = WaveformView(frame: NSRect(x: 16, y: 10, width: width - 32, height: height - 40))
        waveform.autoresizingMask = [.width]
        waveform.isHidden = true

        effect.addSubview(dot)
        effect.addSubview(label)
        effect.addSubview(waveform)
        panel.contentView = effect
    }

    func showRecording() {
        setDot(amber)
        label.stringValue = "Listening…"
        waveform.isHidden = false
        waveform.start()
        present()
    }

    /// Recording an instruction in Command Mode.
    func showCommand() {
        setDot(amber)
        label.stringValue = "Command — speak an instruction"
        waveform.isHidden = false
        waveform.start()
        present()
    }

    /// Mark the current session as hands-free (locked via double-tap).
    func markHandsFree() {
        setDot(amber)
        label.stringValue = "Hands-free · press to stop"
    }

    func showProcessing(preparing: Bool = false) {
        setDot(neutral)
        label.stringValue = preparing ? "Preparing model…" : "Transcribing…"
        waveform.stop()
        waveform.isHidden = true
        present()
    }

    /// Generic working state with a custom label (e.g. "Working…").
    func showWorking(_ text: String) {
        setDot(neutral)
        label.stringValue = text
        waveform.stop()
        waveform.isHidden = true
        present()
    }

    /// Brief transient message (e.g. an error), auto-dismissed.
    func flash(_ message: String) {
        setDot(neutral)
        label.stringValue = message
        waveform.stop()
        waveform.isHidden = true
        present()
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.4) { [weak self] in self?.hide() }
    }

    func hide() {
        waveform.stop()
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
