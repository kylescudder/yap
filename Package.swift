// swift-tools-version: 5.9
import PackageDescription

// Yap — local-first macOS voice dictation (Wispr Flow clone).
// Stage-1 ASR via WhisperKit (on-device CoreML Whisper). MLX + Apple Foundation Models
// cleanup (Stage 2) land in Phase 2.
let package = Package(
    name: "Yap",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(url: "https://github.com/argmaxinc/WhisperKit.git", from: "1.1.0"),
        .package(url: "https://github.com/sparkle-project/Sparkle.git", from: "2.9.5"),
    ],
    targets: [
        .executableTarget(
            name: "Yap",
            dependencies: [
                .product(name: "WhisperKit", package: "WhisperKit"),
                .product(name: "Sparkle", package: "Sparkle"),
            ],
            path: "Sources/Yap"
        )
    ]
)
