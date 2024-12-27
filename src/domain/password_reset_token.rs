use rand::Rng;

const PASSWORD_RESET_TOKEN_MIN_LENGTH: usize = 16;
const PASSWORD_RESET_TOKEN_MAX_LENGTH: usize = 32;

#[derive(Debug)]
pub struct PasswordResetToken(String);

impl PasswordResetToken {
    /// Returns `Self` if input satisfies all name validation constraints, Error otherwise.
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_not_alphanumeric = !s.chars().all(|c| c.is_ascii_alphanumeric());
        let is_incorrect_length =
            s.len() < PASSWORD_RESET_TOKEN_MIN_LENGTH || s.len() > PASSWORD_RESET_TOKEN_MAX_LENGTH;

        if is_empty_or_whitespace || is_incorrect_length || is_not_alphanumeric {
            Err(format!("{s} is not a valid password reset token."))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for PasswordResetToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Default for PasswordResetToken {
    /// Generate a random subscription token of length SUBSCRIPTION_TOKEN_LENGTH, case-sensitive.
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let length =
            rng.gen_range(PASSWORD_RESET_TOKEN_MIN_LENGTH..=PASSWORD_RESET_TOKEN_MAX_LENGTH);
        let token_string = std::iter::repeat_with(|| rng.sample(rand::distributions::Alphanumeric))
            .map(char::from)
            .take(length)
            .collect();

        Self(token_string)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::password_reset_token::{
        PASSWORD_RESET_TOKEN_MAX_LENGTH, PASSWORD_RESET_TOKEN_MIN_LENGTH,
    };

    use super::PasswordResetToken;
    use claims::{assert_err, assert_ok};

    #[test]
    fn generated_tokens_are_parsed_succesfully() {
        let token = PasswordResetToken::default();
        assert_ok!(PasswordResetToken::parse(token.as_ref().to_owned()));
    }

    #[test]
    fn empty_string_is_rejected() {
        let token = "".to_string();
        assert_err!(PasswordResetToken::parse(token));
    }

    #[test]
    fn too_long_is_rejected() {
        let token = "x".repeat(PASSWORD_RESET_TOKEN_MAX_LENGTH + 1);
        assert_err!(PasswordResetToken::parse(token));
    }

    #[test]
    fn correct_length_is_accepted() {
        let mut rng = rand::thread_rng();
        let length = rand::Rng::gen_range(
            &mut rng,
            PASSWORD_RESET_TOKEN_MIN_LENGTH..=PASSWORD_RESET_TOKEN_MAX_LENGTH,
        );
        let token = "x".repeat(length);
        assert_ok!(PasswordResetToken::parse(token));
    }

    #[test]
    fn too_short_is_rejected() {
        let token = "x".repeat(PASSWORD_RESET_TOKEN_MIN_LENGTH - 1);
        assert_err!(PasswordResetToken::parse(token));
    }

    #[test]
    fn not_alphanumeric_rejected() {
        let mut token = "x".repeat(PASSWORD_RESET_TOKEN_MAX_LENGTH - 1);
        token.push('¥');
        assert_err!(PasswordResetToken::parse(token));
    }
}
