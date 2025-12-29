//! WrldBldr Engine - Backend API for TTRPG world management
//!
//! This crate is the *composition root* for the engine.
//! It assembles all adapters, wires them to ports, and starts the server.

mod composition;
mod run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run::run().await
}
