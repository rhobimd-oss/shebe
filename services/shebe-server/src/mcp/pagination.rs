//! Cursor-based pagination for MCP tools
//!
//! Provides opaque cursors for list_dir pagination. Cursors are
//! base64-encoded JSON containing offset, sort mode and a session
//! fingerprint so stale cursors are rejected after reindexing.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::core::storage::SessionMetadata;

/// Cursor for paginating list_dir results.
///
/// Encoded as URL-safe base64 JSON and passed as an opaque string
/// to the MCP client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListDirCursor {
    /// Index of the last item returned in the previous page
    pub last_index: usize,
    /// Sort order used when the cursor was created
    pub sort: String,
    /// Session fingerprint for staleness detection
    pub fingerprint: String,
}

impl ListDirCursor {
    /// Encode cursor as URL-safe base64
    pub fn encode(&self) -> String {
        let json =
            serde_json::to_string(self).expect("ListDirCursor serialization should not fail");
        URL_SAFE_NO_PAD.encode(json.as_bytes())
    }

    /// Decode cursor from URL-safe base64
    pub fn decode(encoded: &str) -> Result<Self, String> {
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| format!("Invalid cursor encoding: {e}"))?;

        let json = String::from_utf8(bytes).map_err(|e| format!("Invalid cursor UTF-8: {e}"))?;

        serde_json::from_str(&json).map_err(|e| format!("Invalid cursor format: {e}"))
    }

    /// Verify this cursor matches the current session state.
    ///
    /// Returns an error message if the fingerprint does not match
    /// (session was reindexed since cursor was created).
    pub fn verify(&self, metadata: &SessionMetadata) -> Result<(), String> {
        let current = session_fingerprint(metadata);
        if self.fingerprint != current {
            return Err("Cursor is stale (session was reindexed). \
                 Start from the first page by omitting the cursor."
                .to_string());
        }
        Ok(())
    }
}

/// Build a lightweight fingerprint from session metadata.
///
/// Format: `{files_indexed}-{chunks_created}-{timestamp}`
/// Changes whenever the session is reindexed.
pub fn session_fingerprint(meta: &SessionMetadata) -> String {
    format!(
        "{}-{}-{}",
        meta.files_indexed,
        meta.chunks_created,
        meta.last_indexed_at.timestamp()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn sample_metadata() -> SessionMetadata {
        SessionMetadata {
            id: "test".to_string(),
            repository_path: PathBuf::from("/test"),
            created_at: Utc::now(),
            last_indexed_at: Utc::now(),
            files_indexed: 314,
            chunks_created: 8741,
            index_size_bytes: 0,
            config: crate::core::storage::SessionConfig::default(),
            schema_version: crate::core::storage::SCHEMA_VERSION,
        }
    }

    #[test]
    fn test_cursor_roundtrip() {
        let meta = sample_metadata();
        let cursor = ListDirCursor {
            last_index: 99,
            sort: "alpha".to_string(),
            fingerprint: session_fingerprint(&meta),
        };

        let encoded = cursor.encode();
        let decoded = ListDirCursor::decode(&encoded).unwrap();
        assert_eq!(cursor, decoded);
    }

    #[test]
    fn test_cursor_decode_invalid_base64() {
        let result = ListDirCursor::decode("!!!not-base64!!!");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cursor encoding"));
    }

    #[test]
    fn test_cursor_decode_invalid_json() {
        let encoded = URL_SAFE_NO_PAD.encode(b"not json");
        let result = ListDirCursor::decode(&encoded);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cursor format"));
    }

    #[test]
    fn test_cursor_verify_matching_fingerprint() {
        let meta = sample_metadata();
        let cursor = ListDirCursor {
            last_index: 0,
            sort: "alpha".to_string(),
            fingerprint: session_fingerprint(&meta),
        };
        assert!(cursor.verify(&meta).is_ok());
    }

    #[test]
    fn test_cursor_verify_stale_fingerprint() {
        let meta = sample_metadata();
        let cursor = ListDirCursor {
            last_index: 0,
            sort: "alpha".to_string(),
            fingerprint: "0-0-0".to_string(),
        };
        let result = cursor.verify(&meta);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("stale"));
    }

    #[test]
    fn test_session_fingerprint_format() {
        let meta = sample_metadata();
        let fp = session_fingerprint(&meta);
        // Should contain the files and chunks count
        assert!(fp.starts_with("314-8741-"));
    }

    #[test]
    fn test_cursor_encode_is_url_safe() {
        let cursor = ListDirCursor {
            last_index: 999,
            sort: "size".to_string(),
            fingerprint: "100-200-1738712345".to_string(),
        };
        let encoded = cursor.encode();
        // URL-safe base64 uses only alphanumeric, hyphen and underscore
        assert!(encoded
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_cursor_last_index_preserved() {
        let meta = sample_metadata();
        let fp = session_fingerprint(&meta);

        // last_index = 0 (lower bound)
        let cursor_zero = ListDirCursor {
            last_index: 0,
            sort: "alpha".to_string(),
            fingerprint: fp.clone(),
        };
        let decoded_zero = ListDirCursor::decode(&cursor_zero.encode()).unwrap();
        assert_eq!(decoded_zero.last_index, 0);

        // last_index = usize::MAX (upper bound)
        let cursor_max = ListDirCursor {
            last_index: usize::MAX,
            sort: "alpha".to_string(),
            fingerprint: fp,
        };
        let decoded_max = ListDirCursor::decode(&cursor_max.encode()).unwrap();
        assert_eq!(decoded_max.last_index, usize::MAX);
    }

    #[test]
    fn test_cursor_sort_field_preserved() {
        let meta = sample_metadata();
        let fp = session_fingerprint(&meta);

        for sort_value in &["alpha", "size", "indexed"] {
            let cursor = ListDirCursor {
                last_index: 42,
                sort: sort_value.to_string(),
                fingerprint: fp.clone(),
            };
            let decoded = ListDirCursor::decode(&cursor.encode()).unwrap();
            assert_eq!(
                decoded.sort, *sort_value,
                "Sort field '{}' did not survive round-trip",
                sort_value
            );
        }
    }

    #[test]
    fn test_cursor_decode_empty_string() {
        let result = ListDirCursor::decode("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should return a descriptive error, not panic
        assert!(
            err.contains("Invalid cursor"),
            "Expected descriptive error, got: {err}"
        );
    }

    #[test]
    fn test_cursor_decode_valid_base64_wrong_schema() {
        // Valid JSON with missing required fields
        let wrong_json = r#"{"unrelated": true}"#;
        let encoded = URL_SAFE_NO_PAD.encode(wrong_json.as_bytes());
        let result = ListDirCursor::decode(&encoded);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Invalid cursor format"),
            "Expected format error for wrong schema, got: {err}"
        );
    }
}
