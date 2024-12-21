use secrecy::{ExposeSecret, Secret};

const PASSWORD_MIN_LENGTH: usize = 8;
const PASSWORD_MAX_LENGTH: usize = 128;

#[derive(Debug, Clone)]
pub struct SubscriberPassword(Secret<String>);

impl SubscriberPassword {
    pub fn parse(s: Secret<String>) -> Result<Self, &'static str> {
        // is_empty_or_whitespace
        if s.expose_secret().trim().is_empty() {
            return Err("Password cannot be empty or only whitespace.");
        }
        // is_ascii_only
        if !s.expose_secret().is_ascii() {
            return Err("Password must contain ASCII characters only");
        }
        // is_too_short
        if s.expose_secret().len() < PASSWORD_MIN_LENGTH {
            let err_str = concat!(
                "Password must be ",
                stringify!(PASSWORD_MIN_LENGTH),
                " characters or longer."
            );
            return Err(err_str);
        }

        // is_too_long
        if s.expose_secret().len() > PASSWORD_MAX_LENGTH {
            let err_str = concat!(
                "Password must be ",
                stringify!(PASSWORD_MIN_LENGTH),
                " characters or shorter."
            );
            return Err(err_str);
        }

        Ok(Self(s))
    }

    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use proptest::{prelude::any, prop_assert, prop_compose, proptest};
    use rand::{prelude::Distribution, rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
    use secrecy::Secret;

    fn generate_valid_password(rng: &mut impl Rng) -> String {
        let lowercase_chars: Vec<char> = ('a'..='z').collect();
        let uppercase_chars: Vec<char> = ('A'..='Z').collect();
        let digit_chars: Vec<char> = ('0'..='9').collect();
        let special_chars: Vec<char> = "!@#$%^&*()_+-=[]{}|;:,.<>?".chars().collect();

        let length = rng.gen_range(PASSWORD_MIN_LENGTH..=PASSWORD_MAX_LENGTH);
        let mut password = String::with_capacity(length);

        // Ensure at least one of each character type
        password.push(*lowercase_chars.choose(rng).unwrap());
        password.push(*uppercase_chars.choose(rng).unwrap());
        password.push(*digit_chars.choose(rng).unwrap());
        password.push(*special_chars.choose(rng).unwrap());

        // Fill the rest with random characters
        let all_chars = [
            &lowercase_chars[..],
            &uppercase_chars[..],
            &digit_chars[..],
            &special_chars[..],
        ]
        .concat();
        let dist = rand::distributions::Uniform::from(0..all_chars.len());

        for _ in password.len()..length {
            password.push(all_chars[dist.sample(rng)]);
        }

        // Shuffle the password to avoid predictable character positioning
        let mut chars: Vec<char> = password.chars().collect();
        chars.shuffle(rng);
        chars.into_iter().collect()
    }

    prop_compose! {
        fn valid_password_strategy()
            (seed in any::<u64>()) -> String {
            let mut rng = StdRng::seed_from_u64(seed);
            generate_valid_password(&mut rng)
        }
    }

    proptest! {
        #[test]
        fn valid_passwords_are_accepted(password in valid_password_strategy()) {
            prop_assert!(SubscriberPassword::parse(Secret::new(password)).is_ok());
        }
    }

    #[test]
    fn a_valid_password_is_accepted() {
        let password = Secret::new("validpass123".to_string());
        assert_ok!(SubscriberPassword::parse(password));
    }

    #[test]
    fn empty_password_is_rejected() {
        let password = Secret::new(String::new());
        assert_err!(SubscriberPassword::parse(password));
    }

    #[test]
    fn whitespace_only_password_is_rejected() {
        let password = Secret::new(String::from(" "));
        assert_err!(SubscriberPassword::parse(password));
    }

    #[test]
    fn password_too_short_is_rejected() {
        let password = Secret::new(String::from("1234567")); // 7 characters
        assert_err!(SubscriberPassword::parse(password));
    }

    #[test]
    fn password_too_long_is_rejected() {
        let password = Secret::new("123456".repeat(25)); // 150 characters
        assert_err!(SubscriberPassword::parse(password));
    }

    #[test]
    fn password_with_non_ascii_is_rejected() {
        let password = Secret::new(String::from("password123ё"));
        assert_err!(SubscriberPassword::parse(password));
    }
}
