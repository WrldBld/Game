//! WrldBldr Engine - Backend API for TTRPG world management
//!
//! This crate is the *composition root* for the engine.
//! It assembles all adapters, wires them to ports, and starts the server.

mod composition;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: Move composition logic here from engine-adapters
    wrldbldr_engine_adapters::run::run().await
}
