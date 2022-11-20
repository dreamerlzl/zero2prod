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
}

impl ToString for UserName {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}
