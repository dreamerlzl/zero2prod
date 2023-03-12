use anyhow::{anyhow, Context};
use argon2::{Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose, Engine};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use secrecy::{ExposeSecret, Secret};
use uuid::Uuid;

use crate::{
    entities::user::{self, Entity as Users},
    utils::spawn_blocking_with_tracing,
};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Authentication failed")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "validate user's credentials", skip(db, credentials))]
pub async fn validate_credentials(
    db: &DatabaseConnection,
    credentials: Credentials,
) -> Result<Uuid, AuthError> {
    let (id, password_hashed) = match get_user_by_credentials(db, &credentials).await {
        Ok(user) => (Some(user.id), user.password_hashed),
        Err(e) => {
            if let AuthError::InvalidCredentials(_) = e {
                (
                    None,
                    "$argon2id$v=19$m=15000,t=2,p=1$\
                    gZiV/M1gPc22E1AH/Jh1Hw$\
                    CWOrkoo7oJBQ/iyh7uJOLO2aLEfrHwTWllSAxTOzRno"
                        .to_owned(),
                )
            } else {
                return Err(e);
            }
        }
    };
    spawn_blocking_with_tracing(|| verify_password(password_hashed, credentials.password))
        .await
        .map_err(|e| anyhow!(format!("hash phc string verify error: {}", e)))??;

    id.ok_or_else(|| anyhow!("unknown username"))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "get user by provided credentials", skip(db, credentials))]
async fn get_user_by_credentials(
    db: &DatabaseConnection,
    credentials: &Credentials,
) -> Result<user::Model, AuthError> {
    let user = Users::find()
        .filter(user::Column::UserName.eq(&credentials.username))
        .one(db)
        .await
        .context("fail to find the auth user")?;

    let user = user
        .ok_or_else(|| anyhow!("invalid username or password"))
        .map_err(AuthError::InvalidCredentials)?;
    Ok(user)
}

#[tracing::instrument(name = "verify password", skip(phc, password))]
fn verify_password(phc: String, password: Secret<String>) -> Result<(), AuthError> {
    let expected_hash = PasswordHash::new(&phc)
        .map_err(|e| anyhow!(format!("fail to extract hash in phc string format: {}", e)))?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &expected_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)?;
    Ok(())
}

pub fn get_hash(input: &str, salt: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = general_purpose::STANDARD.encode(salt);
    // here password_hash is already PHC format
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(input.as_bytes(), &salt)?
    .to_string();
    Ok(password_hash)
}

pub async fn register_test_user(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
    salt: &str,
) -> anyhow::Result<()> {
    let password_hash = get_hash(password, salt).context("fail to register_test_user")?;
    let new_user = user::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_name: ActiveValue::Set(username.to_owned()),
        password_hashed: ActiveValue::Set(password_hash),
    };
    Users::insert(new_user).exec(db).await?;
    Ok(())
}
