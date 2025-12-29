//! WebSocket message handlers organized by domain area
//!
//! Each submodule contains handlers for related ClientMessage variants.

pub mod common;

pub mod challenge;
mod challenge_converters;
pub mod connection;
pub mod inventory;
pub mod misc;
pub mod movement;
pub mod narrative;
pub mod player_action;
pub mod request;
pub mod scene;
pub mod staging;
