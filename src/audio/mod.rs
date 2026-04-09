//! Audio command types for the VZGLYD kernel.
//!
//! This module defines platform-agnostic audio commands that slides can issue
//! through the [`AudioCommand`] enum. Host implementations (native wgpu, WebGPU, etc.)
//! are responsible for actual audio playback using their respective audio APIs.

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

/// Platform-agnostic audio commands issued by slides to the host.
///
/// Slides identify each sound instance with a `u32` ID of their choosing.
/// The host maps this ID to an underlying audio playback handle (e.g., a rodio
/// `Sink` on native, or an `AudioBufferSourceNode` on web).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioCommand {
    /// Play a sound from the slide's embedded assets.
    ///
    /// - `id`: Unique identifier chosen by the slide for this playback instance.
    ///   The slide can later use this ID to stop, pause, or change the volume.
    /// - `asset_key`: The `key` field of a `SoundDesc` from the slide's `sounds` list.
    /// - `volume`: Playback volume from `0.0` (silent) to `1.0` (full volume).
    /// - `looped`: If `true`, the sound repeats until explicitly stopped.
    PlaySound {
        /// Sound instance ID chosen by the slide
        id: u32,
        /// Asset key matching a `SoundDesc.key` in the slide spec
        asset_key: String,
        /// Volume from 0.0 to 1.0
        volume: f32,
        /// Whether to loop the sound
        looped: bool,
    },

    /// Stop a currently playing sound by its ID.
    StopSound {
        /// Sound instance ID to stop
        id: u32,
    },

    /// Change the volume of a playing sound.
    SetVolume {
        /// Sound instance ID
        id: u32,
        /// New volume from 0.0 to 1.0
        volume: f32,
    },

    /// Pause a playing sound (can be resumed with `ResumeSound`).
    PauseSound {
        /// Sound instance ID to pause
        id: u32,
    },

    /// Resume a previously paused sound.
    ResumeSound {
        /// Sound instance ID to resume
        id: u32,
    },
}
