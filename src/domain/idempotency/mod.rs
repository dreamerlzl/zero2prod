mod persistence;
pub use persistence::get_saved_response;

#[derive(Debug)]
pub struct IdempotencyKey(String);

const MAX_LENGTH: usize = 50;

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            anyhow::bail!("The idempotency key shouldn't be empty");
        }
        if s.len() > MAX_LENGTH {
            anyhow::bail!("The idempotency key must be shorter than {MAX_LENGTH} chars");
        }
        Ok(Self(s))
    }
}

impl From<IdempotencyKey> for String {
    fn from(k: IdempotencyKey) -> String {
        k.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
