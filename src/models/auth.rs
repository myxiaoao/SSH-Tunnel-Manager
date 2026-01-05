use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SSH authentication method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthMethod {
    /// Password authentication (not stored)
    Password,
    /// Public key authentication
    PublicKey {
        private_key_path: PathBuf,
        passphrase_required: bool,
    },
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::Password
    }
}

#[allow(dead_code)]
impl AuthMethod {
    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password)
    }

    pub fn is_public_key(&self) -> bool {
        matches!(self, Self::PublicKey { .. })
    }

    pub fn public_key(path: impl Into<PathBuf>, passphrase_required: bool) -> Self {
        Self::PublicKey {
            private_key_path: path.into(),
            passphrase_required,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_serialization() {
        let password_auth = AuthMethod::Password;
        let json = serde_json::to_string(&password_auth).unwrap();
        assert_eq!(json, r#"{"type":"password"}"#);

        let key_auth = AuthMethod::public_key("/path/to/key", false);
        let json = serde_json::to_string(&key_auth).unwrap();
        assert!(json.contains("publickey"));
        assert!(json.contains("/path/to/key"));
    }
}
