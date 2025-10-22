//! Event propagation between clients
//!
//! This module defines events that are propagated between WebRTC clients,
//! such as media data, track information, and keyframe requests.

use std::{sync::Weak, time::Instant};

use str0m::media::{KeyframeRequest, MediaData, Mid};

use crate::model::{client::ClientId, tracks::TrackIn};

/// Events propagated between clients in a multi-peer scenario.
///
/// These events enable sharing of media tracks and data between connected clients,
/// supporting scenarios like broadcasting or relaying.
#[allow(clippy::large_enum_variant)]
pub enum Propagated {
    /// When we have nothing to propagate.
    Noop,

    /// A poll operation has reached its timeout.
    Timeout(Instant),

    /// A new incoming media track has been opened.
    TrackOpen(ClientId, Weak<TrackIn>),

    /// Media data to be propagated from one client to another.
    MediaData(ClientId, MediaData),

    /// A keyframe request from one client to the source.
    KeyframeRequest(ClientId, KeyframeRequest, ClientId, Mid),
}

impl Propagated {
    /// Extracts the client ID from the event, if present.
    ///
    /// # Returns
    ///
    /// * `Some(ClientId)` - If the event is associated with a specific client
    /// * `None` - For events like `Noop` or `Timeout` that aren't client-specific
    pub fn client_id(&self) -> Option<ClientId> {
        match self {
            Propagated::TrackOpen(c, _)
            | Propagated::MediaData(c, _)
            | Propagated::KeyframeRequest(c, _, _, _) => Some(*c),
            _ => None,
        }
    }
}
