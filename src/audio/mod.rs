//! Audio types for the VZGLYD kernel.
//!
//! Sound assets are embedded in the `.vzglyd` bundle. The kernel parses and
//! exposes them via the manifest loader; hosts (native, web) register WASM
//! import functions that slides call directly to control playback.

use serde::{Deserialize, Serialize};

/// Audio format supported for embedded sound assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundFormat {
    /// MP3 audio format
    Mp3,
    /// WAV (PCM) audio format
    Wav,
    /// Ogg Vorbis audio format
    Ogg,
    /// FLAC lossless audio format
    Flac,
}

/// Description of an embedded sound asset in a slide bundle.
///
/// Sound data is embedded directly into the `.vzglyd` bundle alongside
/// textures and meshes, following the same pattern as `TextureDesc`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundDesc {
    /// Unique asset key used to reference this sound (e.g., "notify.mp3")
    pub key: String,
    /// Audio format of the embedded data
    pub format: SoundFormat,
    /// Raw audio bytes (MP3, WAV, Ogg, or FLAC)
    pub data: Vec<u8>,
}
