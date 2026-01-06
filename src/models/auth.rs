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

    #[test]
    fn test_auth_method_default() {
        let auth = AuthMethod::default();
        assert!(matches!(auth, AuthMethod::Password));
    }

    #[test]
    fn test_auth_method_is_password() {
        assert!(AuthMethod::Password.is_password());
        assert!(!AuthMethod::public_key("/path", false).is_password());
    }

    #[test]
    fn test_auth_method_is_public_key() {
        assert!(!AuthMethod::Password.is_public_key());
        assert!(AuthMethod::public_key("/path", false).is_public_key());
    }

    #[test]
    fn test_auth_method_public_key_builder() {
        let auth = AuthMethod::public_key("/home/user/.ssh/id_ed25519", true);

        if let AuthMethod::PublicKey { private_key_path, passphrase_required } = auth {
            assert_eq!(private_key_path, PathBuf::from("/home/user/.ssh/id_ed25519"));
            assert!(passphrase_required);
        } else {
            panic!("Expected PublicKey variant");
        }
    }

    #[test]
    fn test_auth_method_deserialization() {
        let json = r#"{"type":"password"}"#;
        let auth: AuthMethod = serde_json::from_str(json).unwrap();
        assert!(matches!(auth, AuthMethod::Password));

        let json = r#"{"type":"publickey","private_key_path":"/path/to/key","passphrase_required":true}"#;
        let auth: AuthMethod = serde_json::from_str(json).unwrap();
        assert!(matches!(auth, AuthMethod::PublicKey { .. }));
    }

    #[test]
    fn test_auth_method_equality() {
        assert_eq!(AuthMethod::Password, AuthMethod::Password);
        assert_eq!(
            AuthMethod::public_key("/path", true),
            AuthMethod::public_key("/path", true)
        );
        assert_ne!(
            AuthMethod::public_key("/path1", true),
            AuthMethod::public_key("/path2", true)
        );
        assert_ne!(AuthMethod::Password, AuthMethod::public_key("/path", false));
    }

    #[test]
    fn test_auth_method_clone() {
        let original = AuthMethod::public_key("/path/to/key", true);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
