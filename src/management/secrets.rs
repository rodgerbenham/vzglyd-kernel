//! Secrets store for persisting API keys and other sensitive configuration.
//!
//! Secrets are stored as a simple key-value map and serialized to/from JSON.
//! Both the native management server and the web editor work with this type.
//!
//! # Security note
//!
//! The [`SecretsStore`] intentionally exposes values when serialized — callers
//! are responsible for deciding when it is safe to transmit the full store
//! (e.g., only over localhost). Use [`SecretsStore::keys`] when only key names
//! should be surfaced.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Filename used to persist secrets alongside `playlist.json`.
pub const SECRETS_FILENAME: &str = "secrets.json";

/// A flat key-value store for slide secrets such as API keys.
///
/// Serializes to/from a plain JSON object: `{"KEY": "value", ...}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SecretsStore(pub HashMap<String, String>);

impl SecretsStore {
    /// Serialize the store to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.0)
    }

    /// Deserialize a store from a JSON string.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        let map: HashMap<String, String> = serde_json::from_str(s)?;
        Ok(Self(map))
    }

    /// Return the set of key names without exposing values.
    pub fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.0.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }

    /// Merge another store into this one, overwriting any conflicting keys.
    pub fn merge(&mut self, other: SecretsStore) {
        self.0.extend(other.0);
    }

    /// Returns `true` if the store contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let mut store = SecretsStore::default();
        store.0.insert("LASTFM_API_KEY".into(), "abc123".into());
        store.0.insert("OTHER_KEY".into(), "xyz".into());

        let json = store.to_json().expect("serialize");
        let restored = SecretsStore::from_json(&json).expect("deserialize");

        assert_eq!(store, restored);
    }

    #[test]
    fn keys_returns_sorted_names_only() {
        let mut store = SecretsStore::default();
        store.0.insert("Z_KEY".into(), "secret_z".into());
        store.0.insert("A_KEY".into(), "secret_a".into());

        let keys = store.keys();
        assert_eq!(keys, vec!["A_KEY", "Z_KEY"]);
    }

    #[test]
    fn merge_overwrites_existing() {
        let mut base = SecretsStore::default();
        base.0.insert("KEY".into(), "old".into());

        let mut patch = SecretsStore::default();
        patch.0.insert("KEY".into(), "new".into());
        patch.0.insert("OTHER".into(), "extra".into());

        base.merge(patch);

        assert_eq!(base.0["KEY"], "new");
        assert_eq!(base.0["OTHER"], "extra");
        assert_eq!(base.len(), 2);
    }

    #[test]
    fn invalid_json_returns_error() {
        assert!(SecretsStore::from_json("not json").is_err());
    }

    #[test]
    fn non_string_values_return_error() {
        // JSON object with integer value should fail (expects HashMap<String,String>)
        assert!(SecretsStore::from_json(r#"{"KEY": 123}"#).is_err());
    }
}
