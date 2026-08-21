//! Deterministic output policy shared by raw and locally cleaned dictation.

use crate::store::{Settings, Snippet};

/// Applies model-output cleanup, optional courtesy trimming, and snippet expansion.
#[must_use]
pub fn finalize(text: &str, settings: &Settings, snippets: &[Snippet]) -> String {
    let mut result = strip_preamble(text);
    if settings.trim_courtesy {
        result = trim_courtesy(&result);
    }
    for snippet in snippets {
        result = replace_whole_phrase(&result, &snippet.trigger, &snippet.expansion);
    }
    result.trim().to_owned()
}

fn strip_preamble(text: &str) -> String {
    let trimmed = text.trim();
    let Some(colon) = trimmed.find(':') else {
        return trimmed.to_owned();
    };
    let label = trimmed[..colon].trim().to_ascii_lowercase();
    let references_output = ["text", "version", "transcript"]
        .iter()
        .any(|word| label.contains(word));
    let claims_rewrite = [
        "rewritten",
        "cleaned",
        "corrected",
        "revised",
        "formatted",
        "updated",
        "edited",
        "polished",
    ]
    .iter()
    .any(|word| label.contains(word));
    let assistant_lead = ["here is", "here's", "sure", "certainly", "okay", "ok", "of course"]
        .iter()
        .any(|prefix| label.starts_with(prefix));
    if references_output && (claims_rewrite || assistant_lead) {
        trimmed[colon + 1..].trim().to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn trim_courtesy(text: &str) -> String {
    let mut result = text.trim().to_owned();
    result = trim_standalone_courtesy_suffix(&result);
    result = trim_leading_courtesy(&result);
    result = trim_trailing_courtesy(&result);
    capitalize_first(result.trim())
}

fn trim_standalone_courtesy_suffix(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    for suffix in [
        " thank you.",
        " thank you!",
        " thank you so much.",
        " thank you very much.",
        " thanks.",
        " thanks!",
        " many thanks.",
    ] {
        if lower.ends_with(suffix) {
            let start = text.len() - suffix.len();
            if text[..start]
                .chars()
                .next_back()
                .is_some_and(|character| matches!(character, '.' | '!' | '?'))
            {
                return text[..start].trim_end().to_owned();
            }
        }
    }
    text.to_owned()
}

fn trim_leading_courtesy(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    for prefix in ["please, ", "please "] {
        if lower.starts_with(prefix) {
            return text[prefix.len()..].trim_start().to_owned();
        }
    }
    for prefix in [
        "thank you, ",
        "thank you very much, ",
        "thank you so much, ",
        "thanks, ",
        "thanks a lot, ",
        "many thanks, ",
    ] {
        if lower.starts_with(prefix) {
            return text[prefix.len()..].trim_start().to_owned();
        }
    }
    text.to_owned()
}

fn trim_trailing_courtesy(text: &str) -> String {
    let (body, punctuation) = text
        .chars()
        .next_back()
        .filter(|character| matches!(character, '.' | '!' | '?'))
        .map_or((text, ""), |character| {
            (&text[..text.len() - character.len_utf8()], &text[text.len() - character.len_utf8()..])
        });
    let lower = body.to_ascii_lowercase();
    for suffix in [
        ", please",
        " please",
        ", thank you",
        " thank you",
        ", thank you very much",
        " thank you very much",
        ", thank you so much",
        " thank you so much",
        ", thanks",
        " thanks",
        ", thanks a lot",
        " thanks a lot",
        ", many thanks",
        " many thanks",
    ] {
        if lower.ends_with(suffix) {
            return format!("{}{}", body[..body.len() - suffix.len()].trim_end(), punctuation);
        }
    }
    text.to_owned()
}

fn capitalize_first(text: &str) -> String {
    let Some(first) = text.chars().next() else {
        return String::new();
    };
    let mut result = first.to_uppercase().collect::<String>();
    result.push_str(&text[first.len_utf8()..]);
    result
}

fn replace_whole_phrase(text: &str, trigger: &str, expansion: &str) -> String {
    if trigger.is_empty() {
        return text.to_owned();
    }
    let mut matches = Vec::new();
    for (start, _) in text.char_indices() {
        let end = start.saturating_add(trigger.len());
        let Some(candidate) = text.get(start..end) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(trigger) {
            continue;
        }
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if before.is_none_or(|character| !character.is_alphanumeric())
            && after.is_none_or(|character| !character.is_alphanumeric())
        {
            matches.push((start, end));
        }
    }
    let mut result = text.to_owned();
    for (start, end) in matches.into_iter().rev() {
        result.replace_range(start..end, expansion);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{AudioMode, CleanupIntensity};

    fn settings(trim_courtesy: bool) -> Settings {
        Settings {
            cleanup_intensity: CleanupIntensity::Medium,
            audio_mode: AudioMode::Off,
            duck_level: 0.15,
            trim_courtesy,
        }
    }

    #[test]
    fn finalization_matches_portable_macos_policy() {
        let snippets = [Snippet {
            id: 1,
            trigger: "my address".to_owned(),
            expansion: "42 Yap Street".to_owned(),
        }];

        assert_eq!(
            finalize(
                "Here is the rewritten text: please send it to MY ADDRESS, thank you.",
                &settings(true),
                &snippets,
            ),
            "Send it to 42 Yap Street."
        );
    }

    #[test]
    fn legitimate_here_is_sentence_is_preserved() {
        assert_eq!(
            finalize("Here is the plan: buy milk.", &settings(false), &[]),
            "Here is the plan: buy milk."
        );
    }

    #[test]
    fn courtesy_inside_a_real_sentence_is_preserved() {
        assert_eq!(
            finalize("I want to thank you for the update.", &settings(true), &[]),
            "I want to thank you for the update."
        );
    }
}
