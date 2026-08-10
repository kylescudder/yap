import OSLog

/// Tiny logging shim over the unified logging system.
enum Log {
    private static let logger = Logger(subsystem: "com.kyle.yap", category: "app")

    static func info(_ message: String)  { logger.notice("\(message, privacy: .public)") }
    static func error(_ message: String) { logger.error("\(message, privacy: .public)") }
}
