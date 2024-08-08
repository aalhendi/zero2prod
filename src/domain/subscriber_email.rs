use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{s} is not a valid subscriber email."))
        }
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
    use fake::locales::Data;

    #[derive(Debug, Clone)]
    // Custom type. Not using String type as input, since will get all sorts of garbage which will fail validation.
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            // NOTE(aalhendi): reimplimentation of fake::faker::internet::en::SafeEmail
            // `Taken from impl<L: Data + Copy> Dummy<SafeEmail<L>> for String` from <https://docs.rs/fake/2.9.2/src/fake/faker/impls/internet.rs.html#12>
            // let email = SafeEmail().fake_with_rng(g);

            // Choose a name from list of EN first names.
            let username = g.choose(fake::locales::EN::NAME_FIRST_NAME).unwrap();
            let domain = g.choose(&["com", "net", "org"]).unwrap();
            Self(format!("{username}@example.{domain}"))
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
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
