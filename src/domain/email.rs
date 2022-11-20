use validator::validate_email;

#[derive(Debug)]
pub struct Email(String);

impl Email {
    pub fn parse(email: &str) -> Result<Self, String> {
        if validate_email(email) {
            Ok(Self(email.to_owned()))
        } else {
            Err("invalid email".to_owned())
        }
    }
}

impl ToString for Email {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}
