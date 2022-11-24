use serde::Serialize;
use validator::validate_email;

#[derive(Debug, Clone)]
pub struct Email(String);

impl Email {
    pub fn parse(email: String) -> Result<Self, String> {
        if validate_email(&email) {
            Ok(Self(email))
        } else {
            Err("invalid email".to_owned())
        }
    }

    pub fn inner(self) -> String {
        self.0
    }
}

impl Serialize for Email {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl ToString for Email {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}
