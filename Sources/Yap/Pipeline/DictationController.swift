import AppKit
import Combine

/// Orchestrates the dictation loop:
/// hotkey ↓ → capture → (hotkey ↑) → transcribe → clean → snippets → insert at cursor.
///
/// Engines are fixed to the working defaults (WhisperKit + Apple Foundation Models). UI work
/// happens on the main thread; the async transcribe/clean stages run off-main and marshal back.
final class DictationController {
    private let settings: AppSettings
    private let permissions: PermissionsManager
    private let recorder = AudioRecorder()
    private let overlay = DictationOverlay()
    private let hotkey = HotkeyManager()

    private let transcriber: Transcriber = WhisperKitTranscriber()
    private let cleaner: Cleaner = DictationController.makeCleaner()

    private var recording = false
    private var processing = false
    private var transcriberReady = false
    private var commandRecording = false
    private var capturedSelection: String?
    private var cancellables = Set<AnyCancellable>()

    init(settings: AppSettings, permissions: PermissionsManager) {
        self.settings = settings
        self.permissions = permissions

        recorder.onLevel = { [weak self] level in self?.overlay.setLevel(level) }
        hotkey.onStart = { [weak self] in self?.begin() }
        hotkey.onStop = { [weak self] in self?.finish() }
        hotkey.onCancel = { [weak self] in self?.cancel() }
        hotkey.onLock = { [weak self] in self?.overlay.markHandsFree() }
        hotkey.onCommandStart = { [weak self] in self?.beginCommand() }
        hotkey.onCommandStop = { [weak self] in self?.finishCommand() }
        hotkey.setKey(settings.pushToTalkKey)
        hotkey.setCommandKey(settings.commandKey)

        // Warm up in the background (WhisperKit model load/compile; Foundation Models is instant).
        let t = transcriber
        let c = cleaner
        Task {
            await t.prepare()
            await MainActor.run { [weak self] in self?.transcriberReady = true }
        }
        Task { await c.prepare() }

        settings.$pushToTalkKey.dropFirst()
            .sink { [weak self] key in self?.hotkey.setKey(key) }
            .store(in: &cancellables)

        settings.$commandKey.dropFirst()
            .sink { [weak self] key in self?.hotkey.setCommandKey(key) }
            .store(in: &cancellables)

        // Arm the tap as soon as Accessibility is granted (fires with the current value too).
        permissions.$statuses
            .sink { [weak self] statuses in
                if statuses[.accessibility] == .granted { self?.hotkey.start() }
            }
            .store(in: &cancellables)
    }

    private static func makeCleaner() -> Cleaner {
        if #available(macOS 26.0, *) {
            return FoundationModelsCleaner()
        }
        return PassthroughCleaner()
    }

    /// Installs the global push-to-talk tap. Safe to call again after granting Input Monitoring.
    func start() { hotkey.start() }

    // MARK: - Loop

    private func begin() {
        guard !recording, !processing else { return }
        do {
            try recorder.start()
            recording = true
            overlay.showRecording()
        } catch {
            Log.error("Audio start failed: \(error)")
            overlay.flash("Microphone unavailable")
        }
    }

    /// Discard the current recording without transcribing (lone quick tap).
    private func cancel() {
        guard recording else { return }
        recording = false
        _ = recorder.stop()
        overlay.hide()
    }

    // MARK: - Command Mode

    private func beginCommand() {
        guard !recording, !processing, !commandRecording else { return }
        capturedSelection = nil
        // Grab the current selection (⌘C) while the user speaks the instruction.
        SelectionReader.readSelection { [weak self] selection in self?.capturedSelection = selection }
        do {
            try recorder.start()
            commandRecording = true
            overlay.showCommand()
        } catch {
            Log.error("Command audio start failed: \(error)")
            overlay.flash("Microphone unavailable")
        }
    }

    private func finishCommand() {
        guard commandRecording else { return }
        commandRecording = false

        let audio = recorder.stop()
        let selection = capturedSelection
        processing = true
        overlay.showWorking("Working…")

        let transcriber = self.transcriber
        let rate = recorder.sampleRate

        Task { [weak self] in
            guard let self else { return }
            do {
                let instruction = try await transcriber.transcribe(audio, sampleRate: rate).text
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                guard !instruction.isEmpty else {
                    await MainActor.run { self.processing = false; self.overlay.hide() }
                    return
                }
                let result = try await CommandProcessor.process(instruction: instruction, selection: selection)
                await MainActor.run {
                    self.processing = false
                    self.overlay.hide()
                    let out = result.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !out.isEmpty { TextInserter.insert(out) }
                }
            } catch {
                Log.error("Command mode failed: \(error)")
                await MainActor.run { self.processing = false; self.overlay.flash("Command failed") }
            }
        }
    }

    private func finish() {
        guard recording else { return }
        recording = false

        let audio = recorder.stop()
        // Capture the target app now (main thread; overlay is non-activating so focus is unchanged).
        let ctx = ContextCollector.collect()
        processing = true
        overlay.showProcessing(preparing: !transcriberReady)

        let transcriber = self.transcriber
        let cleaner = self.cleaner
        let intensity = settings.cleanupIntensity
        let rate = recorder.sampleRate

        Task { [weak self] in
            guard let self else { return }
            do {
                let transcript = try await transcriber.transcribe(audio, sampleRate: rate)
                let cleaned = try await cleaner.clean(transcript, context: ctx, intensity: intensity)
                await MainActor.run {
                    self.processing = false
                    self.overlay.hide()
                    let trimmed = cleaned.trimmingCharacters(in: .whitespacesAndNewlines)
                    if trimmed.isEmpty {
                        Log.info("Empty transcript — nothing to insert.")
                    } else {
                        let final = SnippetStore.shared.apply(to: cleaned)
                        TextInserter.insert(final)
                        HistoryStore.shared.add(text: final, appName: ctx.appName)
                    }
                }
            } catch {
                Log.error("Pipeline failed: \(error)")
                await MainActor.run {
                    self.processing = false
                    self.overlay.flash("Transcription failed")
                }
            }
        }
    }
}
