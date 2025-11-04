//! Media track management
//!
//! This module provides data structures for managing incoming and outgoing media tracks
//! in WebRTC connections. Tracks can be audio or video streams shared between peers.

use std::{
    sync::{Arc, Weak},
    time::Instant,
};

use str0m::media::{MediaKind, Mid};

use crate::model::client::ClientId;

/// Represents an incoming media track from a remote peer.
///
/// Contains metadata about the track's origin, media ID, and type (audio/video).
#[derive(Debug)]
pub struct TrackIn {
    /// The client ID that originated this track
    pub(crate) origin: ClientId,
    /// The media ID (Mid) assigned to this track
    pub(crate) mid: Mid,
    /// The kind of media (audio or video)
    pub(crate) kind: MediaKind,
}

/// An entry in the track registry with timing information.
///
/// Tracks when keyframe requests were last sent to avoid excessive requests.
#[derive(Debug)]
pub struct TrackInEntry {
    /// The track information wrapped in an Arc for shared ownership
    pub(crate) id: Arc<TrackIn>,
    /// Timestamp of the last keyframe request for this track
    pub(crate) last_keyframe_request: Option<Instant>,
}

/// Represents an outgoing media track being sent to a peer.
///
/// Tracks the negotiation state of outgoing media streams.
#[derive(Debug)]
pub struct TrackOut {
    /// Weak reference to the source track
    pub(crate) track_in: Weak<TrackIn>,
    /// Current state of the outgoing track
    pub(crate) state: TrackOutState,
}

/// The negotiation state of an outgoing track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackOutState {
    /// Track needs to be opened in the next SDP negotiation
    ToOpen,
    /// Track negotiation is in progress with the given Mid
    Negotiating(Mid),
    /// Track is fully negotiated and open for media transmission
    Open(Mid),
}

impl TrackOut {
    /// Gets the media ID (Mid) for this track, if assigned.
    ///
    /// # Returns
    ///
    /// * `Some(Mid)` - If the track is negotiating or open
    /// * `None` - If the track hasn't been assigned a Mid yet
    pub fn mid(&self) -> Option<Mid> {
        match self.state {
            TrackOutState::ToOpen => None,
            TrackOutState::Negotiating(m) | TrackOutState::Open(m) => Some(m),
        }
    }
}
