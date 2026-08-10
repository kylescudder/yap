import AppKit

/// A small floating HUD shown near the bottom of the screen while dictating.
/// Uses a **non-activating** panel so it never steals focus from the target text field.
final class DictationOverlay {
    private let panel: NSPanel
    private let dot = NSView()
    private let label = NSTextField(labelWithString: "Listening…")
    private let waveform: WaveformView

    private let width: CGFloat = 240
    private let height: CGFloat = 58

    init() {
        panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                        styleMask: [.borderless, .nonactivatingPanel],
                        backing: .buffered,
                        defer: false)
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
        dot.layer?.backgroundColor = NSColor.systemRed.cgColor

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

    func showRecording(rainbow: Bool = false) {
        dot.layer?.backgroundColor = NSColor.systemRed.cgColor
        label.stringValue = rainbow ? "Listening… (Max)" : "Listening…"
        waveform.isHidden = false
        waveform.start(rainbow: rainbow)
        present()
    }

    /// Mark the current session as hands-free (locked via double-tap).
    func markHandsFree() {
        dot.layer?.backgroundColor = NSColor.systemRed.cgColor
        label.stringValue = "Hands-free · press to stop"
    }

    /// Recording an instruction in Command Mode.
    func showCommand() {
        dot.layer?.backgroundColor = NSColor.systemPurple.cgColor
        label.stringValue = "Command — speak an instruction"
        waveform.isHidden = false
        waveform.start(rainbow: false)
        present()
    }

    /// Generic working state with a custom label (e.g. "Working…").
    func showWorking(_ text: String) {
        dot.layer?.backgroundColor = NSColor.systemOrange.cgColor
        label.stringValue = text
        waveform.stop()
        waveform.isHidden = true
        present()
    }

    func showProcessing(preparing: Bool = false) {
        dot.layer?.backgroundColor = NSColor.systemOrange.cgColor
        label.stringValue = preparing ? "Preparing model…" : "Transcribing…"
        waveform.stop()
        waveform.isHidden = true
        present()
    }

    /// Brief transient message (e.g. an error), auto-dismissed.
    func flash(_ message: String) {
        dot.layer?.backgroundColor = NSColor.systemGray.cgColor
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

    private func present() {
        positionBottomCenter()
        panel.orderFrontRegardless() // show without activating / stealing focus
    }

    private func positionBottomCenter() {
        guard let screen = NSScreen.main else { return }
        let visible = screen.visibleFrame
        panel.setFrameOrigin(NSPoint(x: visible.midX - width / 2, y: visible.minY + 80))
    }
}
