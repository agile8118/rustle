use crate::error::{AppError, AppResult};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use password_hash::{rand_core::OsRng, SaltString};

pub fn hash(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))
}

pub fn verify(password: &str, phc: &str) -> AppResult<()> {
    let parsed = PasswordHash::new(phc)
        .map_err(|e| AppError::Internal(format!("invalid password hash on record: {e}")))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let h = hash("hunter2-very-strong").unwrap();
        verify("hunter2-very-strong", &h).unwrap();
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let h = hash("correct").unwrap();
        let err = verify("wrong", &h).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }
}
