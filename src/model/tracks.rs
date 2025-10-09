use std::{
    sync::{Arc, Weak},
    time::Instant,
};

use str0m::media::{MediaKind, Mid};

use crate::model::client::ClientId;

#[derive(Debug)]
pub struct TrackIn {
    pub(crate) origin: ClientId,
    pub(crate) mid: Mid,
    pub(crate) kind: MediaKind,
}

#[derive(Debug)]
pub struct TrackInEntry {
    pub(crate) id: Arc<TrackIn>,
    pub(crate) last_keyframe_request: Option<Instant>,
}

#[derive(Debug)]
pub struct TrackOut {
    pub(crate) track_in: Weak<TrackIn>,
    pub(crate) state: TrackOutState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackOutState {
    ToOpen,
    Negotiating(Mid),
    Open(Mid),
}

impl TrackOut {
    pub fn mid(&self) -> Option<Mid> {
        match self.state {
            TrackOutState::ToOpen => None,
            TrackOutState::Negotiating(m) | TrackOutState::Open(m) => Some(m),
        }
    }
}
