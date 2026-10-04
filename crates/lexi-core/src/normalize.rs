use regex::Regex;
use std::sync::OnceLock;

/// Normalizes transcribed speech to isolate the target word or phrase.
/// Removes vocalized commands (e.g., "what does flank mean?", "o que é sneak"),
/// leading/trailing punctuation, quotes, filler words, etc.
pub fn normalize_query(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut text = trimmed.to_lowercase();

    // 1. Strip surrounding quotes or brackets
    text = text
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == '“' || c == '”' || c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}')
        .trim()
        .to_string();

    // 2. Strip common prompt preambles (e.g. if Handy sent a post-processing instruction)
    static PROMPT_PREFIX_RE: OnceLock<Regex> = OnceLock::new();
    let prompt_re = PROMPT_PREFIX_RE.get_or_init(|| {
        Regex::new(r"(?i)^(?:fix\s+.*?:|clean\s+up.*?:|correct\s+.*?:|format\s+.*?:|transcribe.*?:|transcription:)\s*(.*)$").unwrap()
    });
    if let Some(caps) = prompt_re.captures(&text) {
        if let Some(m) = caps.get(1) {
            let inner = m.as_str().trim();
            if !inner.is_empty() {
                text = inner.to_string();
            }
        }
    }

    // 3. Iteratively strip vocal and conversational fillers, prompt preambles and punctuation
    let mut changed = true;
    while changed {
        changed = false;
        let before = text.clone();

        // Trailing conversational suffixes / fillers
        for filler in &[
            " please", " por favor", " pra mim", " para mim",
            " yeah", " yea", " yes", " ok", " okay",
            " man", " bro", " dude",
            " mano", " cara", " vei", " véi",
            " né", " ne", " sabe", " hein", " tipo", " entende",
        ] {
            if text.ends_with(filler) {
                text = text[..text.len() - filler.len()].trim().to_string();
                changed = true;
                break;
            }
        }

        // Leading conversational prefixes / fillers
        for filler in &[
            "um, ", "um ", "uh, ", "uh ", "er, ", "er ", "ah, ", "ah ",
            "tipo, ", "tipo ", "like, ", "like ",
            "então, ", "então ", "entao, ", "entao ",
            "yeah, ", "yeah ", "yea, ", "yea ", "yes, ", "yes ",
            "ok, ", "ok ", "okay, ", "okay ",
            "mano, ", "mano ", "cara, ", "cara ",
            "ei, ", "ei ", "hey, ", "hey ", "ow, ", "ow ",
            "well, ", "well ", "so, ", "so ",
        ] {
            if text.starts_with(filler) {
                text = text[filler.len()..].trim().to_string();
                changed = true;
                break;
            }
        }

        // Punctuation and quotes stripping
        text = text.trim_matches(|c: char| {
            c == ':' || c == ',' || c == '-' || c == '.' || c == '?' || c == '!' || c == ';'
                || c == '"' || c == '\'' || c == '`' || c == '“' || c == '”' || c == '(' || c == ')'
                || c == '[' || c == ']' || c == '{' || c == '}'
        }).trim().to_string();

        if text != before {
            changed = true;
        }
    }

    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        vec![
            // English patterns
            Regex::new(r"^(?:what\s+(?:does|is|are|do)\s+|what's\s+|what\s+mean\s+)(.*?)(?:\s+mean|\s+meaning)?[\?\.]*$").unwrap(),
            Regex::new(r"^(?:meaning\s+of\s+|define\s+|definition\s+of\s+|translate\s+)(.*?)[!\?\.]*$").unwrap(),
            Regex::new(r"^(?:how\s+do\s+you\s+(?:say|translate|spell)\s+|how\s+to\s+say\s+)(.*?)(?:\s+in\s+portuguese)?[\?\.]*$").unwrap(),
            Regex::new(r"^(?:can\s+you\s+translate\s+|could\s+you\s+translate\s+)(.*?)[!\?\.]*$").unwrap(),
            // Portuguese patterns (handling accents, contractions, colons/commas/ellipses)
            Regex::new(r"^(?:(?:o\s*que|oq)\s+(?:significa|quer\s+dizer|eh|é|e|seria)\s*[:,\.\-]*\s*)(.*?)(?:\s+em\s+portugu[eê]s)?[\?\.]*$").unwrap(),
            Regex::new(r"^(?:qual\s+(?:é\s+|e\s+)?o\s+significado\s+(?:de|da\s+palavra|do\s+termo)\s*[:,\.\-]*\s*)(.*?)(?:\s+em\s+portugu[eê]s)?[\?\.]*$").unwrap(),
            Regex::new(r"^(?:significado\s+(?:de|da\s+palavra|do\s+termo)\s*[:,\.\-]*\s*)(.*?)(?:\s+em\s+portugu[eê]s)?[\?\.]*$").unwrap(),
            Regex::new(r"^(?:como\s+(?:se\s+)?(?:traduz|traduzir|fala|diz)\s*[:,\.\-]*\s*)(.*?)(?:\s+em\s+portugu[eê]s|\s+para\s+o\s+portugu[eê]s|\s+pro\s+portugu[eê]s)?[\?\.]*$").unwrap(),
            Regex::new(r"^(?:traduz(?:a|ir)?(?:\s+pra\s+mim|\s+para\s+mim)?\s*[:,\.\-]*\s*)(.*?)(?:\s+em\s+portugu[eê]s|\s+para\s+o\s+portugu[eê]s|\s+pro\s+portugu[eê]s)?[\?\.]*$").unwrap(),
        ]
    });

    for re in patterns {
        if let Some(caps) = re.captures(&text) {
            if let Some(matched) = caps.get(1) {
                let candidate = matched.as_str().trim();
                if !candidate.is_empty() {
                    text = candidate.to_string();
                    break;
                }
            }
        }
    }

    // 4. Strip punctuation around candidate
    text = text.trim_matches(|c: char| c == ':' || c == ',' || c == '-' || c == '.' || c == '"' || c == '\'' || c == '`' || c == '“' || c == '”').trim().to_string();

    // 5. Strip leading English and Portuguese articles & specifiers
    for article in &[
        "the word ", "the term ", "a palavra ", "o termo ",
        "the ", "a ", "an ",
        "o ", "a ", "os ", "as ", "um ", "uma ", "uns ", "umas "
    ] {
        if text.starts_with(article) {
            text = text[article.len()..].trim().to_string();
            break;
        }
    }

    // 6. Strip quotes or brackets around inner word
    text = text.trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == '“' || c == '”' || c == '(' || c == ')' || c == '[' || c == ']').trim().to_string();

    // 7. Strip trailing polite/filler suffixes one more time
    for suffix in &[" please", " por favor", " pra mim", " para mim", " yeah", " yea", " ok", " okay", " né", " ne"] {
        if text.ends_with(suffix) {
            text = text[..text.len() - suffix.len()].trim().to_string();
        }
    }

    // 8. Strip trailing punctuation
    text = text.trim_end_matches(|c: char| c == '?' || c == '.' || c == '!' || c == ',' || c == ':' || c == ';').trim().to_string();

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_word() {
        assert_eq!(normalize_query("Flank"), "flank");
        assert_eq!(normalize_query("  Stealth... "), "stealth");
    }

    #[test]
    fn test_english_questions() {
        assert_eq!(normalize_query("What does flank mean?"), "flank");
        assert_eq!(normalize_query("what's aggro?"), "aggro");
        assert_eq!(normalize_query("define treacherous"), "treacherous");
        assert_eq!(normalize_query("how to say cooldown in portuguese?"), "cooldown");
    }

    #[test]
    fn test_portuguese_questions() {
        assert_eq!(normalize_query("o que significa flank?"), "flank");
        assert_eq!(normalize_query("O que é debuff?"), "debuff");
        assert_eq!(normalize_query("qual o significado de wield"), "wield");
        assert_eq!(normalize_query("traduz bite the bullet"), "bite the bullet");
    }

    #[test]
    fn test_quotes_and_fillers() {
        assert_eq!(normalize_query("\"perish\""), "perish");
        assert_eq!(normalize_query("um, what does heirloom mean?"), "heirloom");
        assert_eq!(normalize_query("what is a sword?"), "sword");
        assert_eq!(normalize_query("o que é um debuff?"), "debuff");
        assert_eq!(normalize_query("the flank please"), "flank");
        assert_eq!(normalize_query("a palavra aggro por favor"), "aggro");
        assert_eq!(normalize_query("Test yeah"), "test");
        assert_eq!(normalize_query("Yeah test"), "test");
        assert_eq!(normalize_query("flank ok?"), "flank");
        assert_eq!(normalize_query("o que é environment mano"), "environment");
        assert_eq!(normalize_query("o que é test né?"), "test");
    }
}
