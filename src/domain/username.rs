use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct UserName(String);

const INVALID_CHARS: [char; 10] = ['(', ')', '{', '}', '<', '>', '\\', '/', '[', ']'];

impl UserName {
    pub fn parse(username: &str) -> Result<Self, String> {
        Self::validate(username)?;
        Ok(Self(username.to_owned()))
    }

    pub fn validate(username: &str) -> Result<(), String> {
        if username.trim().is_empty() {
            return Err("empty username is not allowed".to_owned());
        }
        if username.graphemes(true).count() > 256 {
            return Err("username too long".to_owned());
        }
        if username.chars().any(|c| INVALID_CHARS.contains(&c)) {
            return Err("username contains invalid char".to_owned());
        }
        Ok(())
    }

    pub fn inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ToString for UserName {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_username() {
        assert!(UserName::validate("").is_err());
    }

    #[test]
    fn test_length() {
        assert!(UserName::validate(&"ά".repeat(256)).is_ok());
        assert!(UserName::validate(&"ά".repeat(257)).is_err());
    }

    #[test]
    fn whitespaces_only() {
        assert!(UserName::validate(&" ".repeat(3)).is_err());
    }

    #[test]
    fn empty_string() {
        assert!(UserName::validate("").is_err());
    }

    #[test]
    fn valid_names() {
        let names = ["Wright Lin", "JustForYou", "a   b c@"];
        for name in names {
            assert!(UserName::validate(name).is_ok());
        }
    }
}
