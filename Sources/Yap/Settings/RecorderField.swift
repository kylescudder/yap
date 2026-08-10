import SwiftUI

/// A "click to record" control that captures a key/chord into a `KeyBinding`.
struct RecorderField: View {
    let title: String
    @Binding var binding: KeyBinding

    @State private var recording = false
    @State private var recorder = HotkeyRecorder()

    var body: some View {
        HStack {
            Text(title)
            Spacer()
            Button(action: toggle) {
                Text(recording ? "Press a key… (Esc cancels)" : binding.display)
                    .frame(minWidth: 150)
            }
        }
    }

    private func toggle() {
        if recording {
            recorder.stop()
            recording = false
        } else {
            recording = true
            recorder.onCapture = { captured in
                if let captured { binding = captured }
                recording = false
            }
            recorder.start()
        }
    }
}
