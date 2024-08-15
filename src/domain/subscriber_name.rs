use unicode_segmentation::UnicodeSegmentation;

const NAME_MAX_LENGTH: usize = 256;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    /// Returns `Self` if input satisfies all name validation constraints, Error otherwise.
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        // Use extended grapheme definition set (recommended)
        let is_too_long = s.graphemes(true).count() > NAME_MAX_LENGTH;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{s} is not a valid subscriber name."))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    /// Caller shared ref to inner string. gives caller **read-only** access
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{subscriber_name::NAME_MAX_LENGTH, SubscriberName};
    use claims::{assert_err, assert_ok};
    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ё".repeat(NAME_MAX_LENGTH);
        assert_ok!(SubscriberName::parse(name));
    }
    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(NAME_MAX_LENGTH + 1);
        assert_err!(SubscriberName::parse(name));
    }
    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }
    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }
    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }
    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "John GSD Doe".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
