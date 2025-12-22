//! WrldBldr Engine - Backend API for TTRPG world management
//!
//! This crate is the *composition root* for the engine.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wrldbldr_engine_adapters::run::run().await
}
