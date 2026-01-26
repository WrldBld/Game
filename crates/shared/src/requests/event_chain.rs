use serde::{Deserialize, Serialize};

use super::{CreateEventChainData, UpdateEventChainData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventChainRequest {
    ListEventChains {
        world_id: String,
    },
    GetEventChain {
        chain_id: String,
    },
    CreateEventChain {
        world_id: String,
        data: CreateEventChainData,
    },
    UpdateEventChain {
        chain_id: String,
        data: UpdateEventChainData,
    },
    DeleteEventChain {
        chain_id: String,
    },
    SetEventChainActive {
        chain_id: String,
        active: bool,
    },
    SetEventChainFavorite {
        chain_id: String,
        favorite: bool,
    },
    AddEventToChain {
        chain_id: String,
        event_id: String,
        #[serde(default)]
        position: Option<u32>,
    },
    RemoveEventFromChain {
        chain_id: String,
        event_id: String,
    },
    CompleteChainEvent {
        chain_id: String,
        event_id: String,
    },
    ResetEventChain {
        chain_id: String,
    },
    GetEventChainStatus {
        chain_id: String,
    },
}
