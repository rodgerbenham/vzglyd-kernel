//! Slide library metadata for listing available `.vzglyd` bundles.

use serde::{Deserialize, Serialize};

use crate::manifest::SlideManifest;

/// Metadata for a single `.vzglyd` file in the slide library.
///
/// Returned by the management server's `GET /api/slides` endpoint and
/// consumed by both the native management UI and the web editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideLibraryEntry {
    /// Path of the bundle relative to the slides directory, e.g. `"afl.vzglyd"`.
    pub path: String,
    /// Size of the bundle archive in bytes.
    pub size_bytes: u64,
    /// Manifest extracted from the bundle, if it could be parsed.
    /// `None` indicates a corrupt or unreadable bundle.
    pub manifest: Option<SlideManifest>,
}
