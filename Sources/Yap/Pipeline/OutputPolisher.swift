import Foundation

/// Deterministic post-processing applied to cleaned dictation before it's inserted:
///   • `stripPreamble` — removes an assistant-style label the cleanup model sometimes prepends
///     ("Here is the rewritten text:"), without touching legitimately dictated text.
///   • `trimCourtesy` — drops "please" / "thank you" at sentence *boundaries* (leading, trailing,
///     or a standalone sentence) while keeping them mid-sentence.
enum OutputPolisher {

    /// Courtesy phrases trimmed at sentence boundaries.
    private static let court =
        #"(?:please|thank you(?: very much| so much)?|thanks(?: a lot)?|many thanks)"#

    // MARK: - Preamble

    /// Strip a leading "Here is the rewritten text:"-style label. Both patterns require the label to
    /// reference the *text/version/transcript* AND end in a colon, so a real sentence like
    /// "Here is the plan: buy milk." is left untouched.
    static func stripPreamble(_ text: String) -> String {
        var s = text
        let patterns = [
            #"(?is)^\s*(?:sure|certainly|okay|ok|of course)?[,!.]*\s*here(?:'s| is| are)\s+(?:the\s+)?(?:rewritten|cleaned|corrected|revised|formatted|updated|edited|polished)?\s*(?:text|version|transcript)\b[^\n:]*:\s*"#,
            #"(?is)^\s*(?:the\s+)?(?:rewritten|cleaned|corrected|revised|formatted|updated|edited|polished)\s+(?:text|version|transcript)\s*:\s*"#,
        ]
        for p in patterns { s = replace(s, p, with: "") }
        return s.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // MARK: - Courtesy

    static func trimCourtesy(_ text: String) -> String {
        var s = text
        // Standalone courtesy sentences → gone ("Send it. Thank you." → "Send it.").
        s = replace(s, #"(?i)(?:^|(?<=[.!?]))\s*"# + court + #"\s*[.!?]+"#, with: "")
        // Leading courtesy before real content → drop it, capitalize what follows.
        s = trimLeading(s)
        // Trailing courtesy appended to a sentence → drop it ("Send it, please." → "Send it.").
        s = replace(s, #"(?i)[ ,;]+"# + court + #"\b\s*(?=[.!?]|$)"#, with: "")
        // Tidy the seams.
        s = replace(s, #"[ \t]{2,}"#, with: " ")
        s = replace(s, #"\s+([.!?,;])"#, with: "$1")
        return capitalizingFirst(s.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// Remove courtesy at a sentence start and upper-case the following letter.
    ///
    /// "please" is stripped whether or not a comma follows ("Please send it" → "Send it").
    /// Leading "thank you"/"thanks" is stripped ONLY when set off by a comma ("Thanks, send it"),
    /// so a real phrase like "Thank you for the update" is left intact.
    private static func trimLeading(_ text: String) -> String {
        var s = text
        s = capitalizeAfterLead(s, #"(?i)(^|[.!?]\s+)please,?[ \t]+(\p{L})"#)
        s = capitalizeAfterLead(s, #"(?i)(^|[.!?]\s+)(?:thank you(?: very much| so much)?|thanks(?: a lot)?|many thanks),[ \t]+(\p{L})"#)
        return s
    }

    /// Delete the courtesy phrase matched by `pattern` (group 1 = sentence lead-in, group 2 = the
    /// next letter) and upper-case that letter.
    private static func capitalizeAfterLead(_ text: String, _ pattern: String) -> String {
        guard let re = try? NSRegularExpression(pattern: pattern) else { return text }
        let source = text as NSString
        let mutable = NSMutableString(string: text)
        // Apply matches back-to-front so earlier ranges stay valid.
        let matches = re.matches(in: text, range: NSRange(location: 0, length: source.length))
        for match in matches.reversed() {
            let lead = source.substring(with: match.range(at: 1))
            let letter = source.substring(with: match.range(at: 2)).uppercased()
            mutable.replaceCharacters(in: match.range, with: lead + letter)
        }
        return mutable as String
    }

    // MARK: - Helpers

    private static func replace(_ text: String, _ pattern: String, with template: String) -> String {
        guard let re = try? NSRegularExpression(pattern: pattern) else { return text }
        let range = NSRange(text.startIndex..., in: text)
        return re.stringByReplacingMatches(in: text, range: range, withTemplate: template)
    }

    private static func capitalizingFirst(_ s: String) -> String {
        guard let first = s.first else { return s }
        return first.uppercased() + s.dropFirst()
    }
}
