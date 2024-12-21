use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if ValidateEmail::validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{s} is not a valid subscriber email."))
        }
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Forward to Display impl of wrapped String
        self.0.fmt(f)
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claims::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use proptest::{prelude::any, prop_assert, prop_compose, proptest};
    use rand::{rngs::StdRng, SeedableRng};

    prop_compose! {
        fn valid_email_strategy()
            // PropTest and `fake` interoperate through the `rand` crate.
            // The strategy takes a random u64 seed from PropTest's built-in generators,
            // uses it to create a deterministic random number generator (StdRng),
            // then passes that to `fake`'s SafeEmail generator.
            // This ensures reproducible email generation for each test case
            // while maintaining the randomness properties we want for testing.
            (seed in any::<u64>()) -> String {
            let mut rng = StdRng::seed_from_u64(seed);
            Fake::fake_with_rng(&SafeEmail(), &mut rng)
        }
    }

    proptest! {
        #[test]
        fn valid_emails_are_parsed_successfully(email in valid_email_strategy()) {
            prop_assert!(SubscriberEmail::parse(email).is_ok());
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "johndomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
}
