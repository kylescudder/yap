import AppKit
import CoreAudio

/// Manages other apps' audio while you dictate — either *lowering* the system volume or *pausing*
/// playback — always with a short fade so it never snaps.
///
/// Detection (`isPlaying`) must be sampled **before** the mic starts: `AVAudioEngine` spins up the
/// default *output* device, which would otherwise make us think music is playing when it isn't.
/// All volume work runs on a private serial queue; `restore()` is a safe no-op when nothing's active.
final class AudioController {
    private let queue = DispatchQueue(label: "com.yap.audio")
    private var savedVolume: Float32?   // only touched on `queue`
    private var ducked = false
    private var didPause = false
    private var token = 0               // supersedes an in-flight fade

    /// Is any process currently outputting audio to the default output device? Sample this BEFORE
    /// starting the recorder.
    static func isPlaying() -> Bool {
        guard let dev = device() else { return false }
        return isRunningSomewhere(dev)
    }

    /// `.lower` mode — glide down to `target` (0 = mute … 1 = unchanged).
    func duck(to target: Float32) {
        queue.async { [weak self] in self?._duck(to: target) }
    }

    /// `.pause` mode — fade out, then pause the Now-Playing app.
    func pause() {
        queue.async { [weak self] in self?._pause() }
    }

    /// Undo whichever action ran. Safe to call even if nothing is active.
    func restore() {
        queue.async { [weak self] in self?._restore() }
    }

    // MARK: - Serial-queue bodies

    private func _duck(to target: Float32) {
        guard !ducked, let dev = Self.device(), let current = Self.volume(dev) else { return }
        savedVolume = current
        ducked = true
        token += 1
        ramp(dev, from: current, to: min(current, clamp(target)), ms: 120, token: token)
    }

    private func _pause() {
        guard !didPause, let dev = Self.device(), let current = Self.volume(dev) else { return }
        savedVolume = current
        didPause = true
        token += 1
        ramp(dev, from: current, to: 0, ms: 140, token: token)   // fade out
        MediaKey.playPause()                                     // pause (now silent)
        Self.setVolume(dev, current)                             // reset slider — inaudible while paused
    }

    private func _restore() {
        guard let dev = Self.device() else { ducked = false; didPause = false; savedVolume = nil; return }
        if didPause, let saved = savedVolume {
            didPause = false
            token += 1
            Self.setVolume(dev, 0)          // start silent
            MediaKey.playPause()            // resume
            ramp(dev, from: 0, to: saved, ms: 180, token: token)   // fade in
        } else if ducked, let saved = savedVolume {
            ducked = false
            token += 1
            let current = Self.volume(dev) ?? saved
            ramp(dev, from: current, to: saved, ms: 140, token: token)
        }
        savedVolume = nil
    }

    /// Stepped glide (~20 ms/step). Runs on the serial queue, so fades never overlap.
    private func ramp(_ dev: AudioDeviceID, from: Float32, to: Float32, ms: Int, token t: Int) {
        let steps = max(1, ms / 20)
        for i in 1...steps {
            guard token == t else { return }   // superseded
            let frac = Float32(i) / Float32(steps)
            Self.setVolume(dev, from + (to - from) * frac)
            usleep(useconds_t((ms * 1000) / steps))
        }
    }

    private func clamp(_ v: Float32) -> Float32 { max(0, min(1, v)) }

    // MARK: - CoreAudio plumbing

    private static func device() -> AudioDeviceID? {
        var dev = AudioDeviceID(0)
        var size = UInt32(MemoryLayout<AudioDeviceID>.size)
        var addr = AudioObjectPropertyAddress(
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain)
        let ok = AudioObjectGetPropertyData(
            AudioObjectID(kAudioObjectSystemObject), &addr, 0, nil, &size, &dev) == noErr
        return (ok && dev != 0) ? dev : nil
    }

    private static func isRunningSomewhere(_ dev: AudioDeviceID) -> Bool {
        var running = UInt32(0)
        var size = UInt32(MemoryLayout<UInt32>.size)
        var addr = AudioObjectPropertyAddress(
            mSelector: kAudioDevicePropertyDeviceIsRunningSomewhere,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain)
        guard AudioObjectGetPropertyData(dev, &addr, 0, nil, &size, &running) == noErr else { return false }
        return running != 0
    }

    private static func volume(_ dev: AudioDeviceID) -> Float32? {
        for element in [kAudioObjectPropertyElementMain, AudioObjectPropertyElement(1)] {
            var addr = AudioObjectPropertyAddress(
                mSelector: kAudioDevicePropertyVolumeScalar,
                mScope: kAudioObjectPropertyScopeOutput, mElement: element)
            guard AudioObjectHasProperty(dev, &addr) else { continue }
            var vol = Float32(0)
            var size = UInt32(MemoryLayout<Float32>.size)
            if AudioObjectGetPropertyData(dev, &addr, 0, nil, &size, &vol) == noErr { return vol }
        }
        return nil
    }

    private static func setVolume(_ dev: AudioDeviceID, _ value: Float32) {
        let v = max(0, min(1, value))
        for element in [kAudioObjectPropertyElementMain, AudioObjectPropertyElement(1), AudioObjectPropertyElement(2)] {
            var addr = AudioObjectPropertyAddress(
                mSelector: kAudioDevicePropertyVolumeScalar,
                mScope: kAudioObjectPropertyScopeOutput, mElement: element)
            var settable = DarwinBoolean(false)
            guard AudioObjectHasProperty(dev, &addr),
                  AudioObjectIsPropertySettable(dev, &addr, &settable) == noErr, settable.boolValue
            else { continue }
            var vv = v
            AudioObjectSetPropertyData(dev, &addr, 0, nil, UInt32(MemoryLayout<Float32>.size), &vv)
            if element == kAudioObjectPropertyElementMain { return }   // main covers all channels
        }
    }
}

/// Posts the system Play/Pause media key — the same event the keyboard's ▶︎⏸ key sends, routed to
/// whatever app is currently Now-Playing (Music, Spotify, Safari/Chrome media, …).
enum MediaKey {
    // NX_KEYTYPE_PLAY from IOKit/hidsystem/ev_keymap.h
    private static let play = 16

    static func playPause() {
        post(down: true)
        post(down: false)
    }

    private static func post(down: Bool) {
        let flags: UInt = down ? 0xA00 : 0xB00
        let data1 = (play << 16) | ((down ? 0xA : 0xB) << 8)
        guard let event = NSEvent.otherEvent(
            with: .systemDefined,
            location: .zero,
            modifierFlags: NSEvent.ModifierFlags(rawValue: flags),
            timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: 0,
            context: nil,
            subtype: 8,          // NX_SUBTYPE_AUX_CONTROL_BUTTONS
            data1: data1,
            data2: -1
        ) else { return }
        event.cgEvent?.post(tap: .cghidEventTap)
    }
}
