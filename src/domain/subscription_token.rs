use rand::Rng;

const SUBSCRIPTION_TOKEN_LENGTH: usize = 25;

#[derive(Debug)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    /// Returns `Self` if input satisfies all name validation constraints, Error otherwise.
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_not_alphanumeric = !s.chars().all(|c| c.is_ascii_alphanumeric());
        let is_incorrect_length = s.len() != SUBSCRIPTION_TOKEN_LENGTH;

        if is_empty_or_whitespace || is_incorrect_length || is_not_alphanumeric {
            Err(format!("{s} is not a valid subscription token."))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Default for SubscriptionToken {
    /// Generate a random subscription token of length SUBSCRIPTION_TOKEN_LENGTH, case-sensitive.
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let token_string = std::iter::repeat_with(|| rng.sample(rand::distributions::Alphanumeric))
            .map(char::from)
            .take(SUBSCRIPTION_TOKEN_LENGTH)
            .collect();

        Self(token_string)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::subscription_token::SUBSCRIPTION_TOKEN_LENGTH;

    use super::SubscriptionToken;
    use claims::{assert_err, assert_ok};

    #[test]
    fn generated_tokens_are_parsed_succesfully() {
        let token = SubscriptionToken::default();
        assert_ok!(SubscriptionToken::parse(token.as_ref().to_owned()));
    }

    #[test]
    fn empty_string_is_rejected() {
        let token = "".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn too_long_is_rejected() {
        let token = "x".repeat(SUBSCRIPTION_TOKEN_LENGTH + 1);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn correct_length_is_accepted() {
        let token = "x".repeat(SUBSCRIPTION_TOKEN_LENGTH);
        assert_ok!(SubscriptionToken::parse(token));
    }

    #[test]
    fn too_short_is_rejected() {
        let token = "x".repeat(SUBSCRIPTION_TOKEN_LENGTH - 1);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn not_alphanumeric_rejected() {
        let mut token = "x".repeat(SUBSCRIPTION_TOKEN_LENGTH - 1);
        token.push('¥');
        assert_err!(SubscriptionToken::parse(token));
    }
}
