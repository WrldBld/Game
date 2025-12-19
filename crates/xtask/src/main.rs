use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("arch-check") => arch_check(),
        Some(cmd) => anyhow::bail!("Unknown xtask command: {cmd}"),
        None => anyhow::bail!("Usage: cargo xtask <command>\n\nCommands:\n  arch-check"),
    }
}

fn arch_check() -> anyhow::Result<()> {
    // Minimal placeholder; actual check implemented after crates are wired.
    // We keep it functional so `cargo run -p xtask -- arch-check` works.
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("running cargo metadata")?;

    if !output.status.success() {
        anyhow::bail!("cargo metadata failed")
    }

    Ok(())
}
