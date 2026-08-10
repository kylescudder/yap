import AppKit
import CoreGraphics
import ApplicationServices

/// Global hotkeys via a `CGEventTap`. Watches a dictation binding (hold / double-tap-to-lock) and a
/// Command Mode binding (hold). Modifier-only bindings are observed via `flagsChanged` and never
/// consumed; regular-key / chord bindings are matched on `keyDown`/`keyUp` and **consumed** so the
/// key doesn't type. Requires the Input Monitoring permission.
final class HotkeyManager {
    var onStart: (() -> Void)?   // begin a recording session
    var onStop: (() -> Void)?    // finalize + transcribe
    var onCancel: (() -> Void)?  // discard (lone quick tap)
    var onLock: (() -> Void)?    // entered hands-free (double-tap)
    var onCommandStart: (() -> Void)?
    var onCommandStop: (() -> Void)?

    private var tap: CFMachPort?
    private var runLoopSource: CFRunLoopSource?

    private var dictationBinding: KeyBinding = .pushToTalkDefault
    private var commandBinding: KeyBinding = .commandDefault

    // Dictation hold/tap/double-tap state.
    private var isDown = false
    private var sessionActive = false
    private var locked = false
    private var ignoreNextUp = false
    private var pressDownTime: Date?
    private var lastDownTime: Date?
    private var pendingCancelTimer: Timer?
    private let holdThreshold: TimeInterval = 0.35
    private let doubleTapWindow: TimeInterval = 0.5

    // Command Mode hold state.
    private var commandDown = false

    var isInstalled: Bool { tap != nil }

    func setKey(_ binding: KeyBinding) {
        dictationBinding = binding
        resetState()
        Log.info("Push-to-talk: \(binding.display)")
    }

    func setCommandKey(_ binding: KeyBinding) {
        commandBinding = binding
        commandDown = false
        Log.info("Command key: \(binding.display)")
    }

    private func resetState() {
        isDown = false
        sessionActive = false
        locked = false
        ignoreNextUp = false
        pressDownTime = nil
        lastDownTime = nil
        pendingCancelTimer?.invalidate()
        pendingCancelTimer = nil
    }

    func start() {
        guard tap == nil else { return }

        // The active event tap is authorized by Accessibility. Create it only once that's
        // granted (otherwise tapCreate can hand back a dead tap that never gets events).
        guard AXIsProcessTrusted() else {
            Log.info("Accessibility not granted yet — hotkey will arm once it is.")
            return
        }

        let mask: CGEventMask =
            (1 << CGEventType.flagsChanged.rawValue) |
            (1 << CGEventType.keyDown.rawValue) |
            (1 << CGEventType.keyUp.rawValue)
        let refcon = Unmanaged.passUnretained(self).toOpaque()

        guard let tap = CGEvent.tapCreate(
            tap: .cgSessionEventTap,
            place: .headInsertEventTap,
            options: .defaultTap, // active tap so we can consume matched regular keys
            eventsOfInterest: mask,
            callback: hotkeyTapCallback,
            userInfo: refcon
        ) else {
            Log.error("Event tap creation failed.")
            return
        }

        self.tap = tap
        let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
        runLoopSource = source
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: tap, enable: true)
        Log.info("Push-to-talk armed.")
    }

    func stop() {
        if let source = runLoopSource {
            CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .commonModes)
        }
        if let tap { CGEvent.tapEnable(tap: tap, enable: false) }
        tap = nil
        runLoopSource = nil
        resetState()
    }

    /// Returns true to consume (swallow) the event.
    fileprivate func handle(type: CGEventType, event: CGEvent) -> Bool {
        if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
            if let tap { CGEvent.tapEnable(tap: tap, enable: true) }
            return false
        }

        let keycode = event.getIntegerValueField(.keyboardEventKeycode)

        switch type {
        case .flagsChanged:
            // Modifier-only bindings — observed, never consumed.
            if dictationBinding.isModifierOnly, keycode == Int64(dictationBinding.keyCode) {
                dictationPressed((event.flags.rawValue & dictationBinding.modifiers) != 0, isRepeat: false)
            } else if commandBinding.isModifierOnly, keycode == Int64(commandBinding.keyCode) {
                commandPressed((event.flags.rawValue & commandBinding.modifiers) != 0)
            }
            return false

        case .keyDown:
            let isRepeat = event.getIntegerValueField(.keyboardEventAutorepeat) != 0
            if !dictationBinding.isModifierOnly, matchesRegular(event, dictationBinding, keycode: keycode) {
                dictationPressed(true, isRepeat: isRepeat)
                return true
            }
            if !commandBinding.isModifierOnly, matchesRegular(event, commandBinding, keycode: keycode) {
                if !isRepeat { commandPressed(true) }
                return true
            }
            return false

        case .keyUp:
            // Match release by keycode only (modifiers may already be released).
            if !dictationBinding.isModifierOnly, keycode == Int64(dictationBinding.keyCode) {
                dictationPressed(false, isRepeat: false)
                return true
            }
            if !commandBinding.isModifierOnly, keycode == Int64(commandBinding.keyCode) {
                commandPressed(false)
                return true
            }
            return false

        default:
            return false
        }
    }

    private func matchesRegular(_ event: CGEvent, _ binding: KeyBinding, keycode: Int64) -> Bool {
        guard keycode == Int64(binding.keyCode) else { return false }
        let eventMods = event.flags.intersection(KeyBinding.relevantModifiers).rawValue
        return eventMods == binding.modifiers
    }

    private func dictationPressed(_ pressed: Bool, isRepeat: Bool) {
        if isRepeat { return }
        if pressed {
            guard !isDown else { return }
            isDown = true
            keyDown()
        } else {
            guard isDown else { return }
            isDown = false
            keyUp()
        }
    }

    private func commandPressed(_ pressed: Bool) {
        if pressed {
            guard !commandDown else { return }
            commandDown = true
            onCommandStart?()
        } else {
            guard commandDown else { return }
            commandDown = false
            onCommandStop?()
        }
    }

    // MARK: - Dictation hold / tap / double-tap

    private func keyDown() {
        let now = Date()
        let isDoubleTap = lastDownTime.map { now.timeIntervalSince($0) <= doubleTapWindow } ?? false
        lastDownTime = now
        pendingCancelTimer?.invalidate(); pendingCancelTimer = nil

        if locked {
            locked = false
            sessionActive = false
            ignoreNextUp = true
            onStop?()
            return
        }
        if isDoubleTap {
            if !sessionActive { sessionActive = true; onStart?() }
            locked = true
            onLock?()
            return
        }
        if !sessionActive { sessionActive = true; onStart?() }
        pressDownTime = now
    }

    private func keyUp() {
        if ignoreNextUp { ignoreNextUp = false; return }
        guard sessionActive, !locked else { return }

        let duration = Date().timeIntervalSince(pressDownTime ?? Date())
        if duration >= holdThreshold {
            sessionActive = false
            onStop?()
        } else {
            schedulePendingCancel()
        }
    }

    private func schedulePendingCancel() {
        pendingCancelTimer?.invalidate()
        let timer = Timer(timeInterval: doubleTapWindow, repeats: false) { [weak self] _ in
            guard let self else { return }
            self.sessionActive = false
            self.pendingCancelTimer = nil
            self.onCancel?()
        }
        RunLoop.main.add(timer, forMode: .common)
        pendingCancelTimer = timer
    }
}

/// C-compatible tap callback (captures nothing; `self` via refcon). Returns nil to swallow.
private let hotkeyTapCallback: CGEventTapCallBack = { _, type, event, refcon in
    if let refcon {
        let consume = Unmanaged<HotkeyManager>.fromOpaque(refcon)
            .takeUnretainedValue()
            .handle(type: type, event: event)
        if consume { return nil }
    }
    return Unmanaged.passUnretained(event)
}
