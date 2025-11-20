//! Migrating an application from anyhow to rootcause
//!
//! This example shows how to gradually adopt rootcause in an existing
//! anyhow-based codebase. We'll migrate a small application that uses
//! a key-value store with metrics collection.
//!
//! # Migration Strategies
//!
//! You can migrate in any order that makes sense for your codebase:
//!
//! - **Top-down** (application first): Use rootcause features immediately in
//!   your business logic. Call anyhow dependencies with `.into_rootcause()`.
//!
//! - **Bottom-up** (libraries first): Adopt rootcause internally while keeping
//!   anyhow-compatible public APIs using `.into_anyhow()`.
//!
//! - **Middle-out**: Not recommended - requires conversions in both directions.
//!
//! **Which to choose?** Start where you'll get the most value from rootcause's
//! features, and prefer loosely-connected code (leaf libraries or top-level
//! binaries). For most users, **starting top-down** is easiest - you get
//! immediate benefits and only need `.into_rootcause()` conversions.
//!
//! # This Example's Approach
//!
//! We demonstrate a **combination approach**: start top-down with the
//! application, then gradually convert dependencies. This is realistic when
//! you control all the code but want to be cautious about breaking changes.
//!
//! The example treats each component as if it came from a separate crate:
//! - `metrics`: A metrics collection library (pretend you depend on this)
//! - `kv_store`: A key-value store (pretend you depend on this too)
//! - Your application: The code you're migrating
//!
//! # Migration Stages
//!
//! Each stage is in its own file so you can open them side-by-side for
//! comparison:
//!
//! 1. **Stage 1** (`stage1.rs`): Everything uses anyhow (starting point)
//! 2. **Stage 2** (`stage2.rs`): Application converted to rootcause
//! 3. **Stage 3** (`stage3.rs`): Metrics library converted internally
//! 4. **Stage 4** (`stage4.rs`): KV store converted internally
//! 5. **Stage 5** (`stage5.rs`): Full migration complete (breaking change to
//!    public APIs)

// =============================================================================
// Stage 1: Everything uses anyhow
// =============================================================================
#[path = "anyhow_migration/stage1.rs"]
pub mod v1_original_anyhow;

// =============================================================================
// Stage 2: Application converted to rootcause
// =============================================================================
#[path = "anyhow_migration/stage2.rs"]
pub mod v2_main_converted;

// =============================================================================
// Stage 3: Metrics library converted internally
// =============================================================================
#[path = "anyhow_migration/stage3.rs"]
pub mod v3_metrics_converted;

// =============================================================================
// Stage 4: KV store converted internally
// =============================================================================
#[path = "anyhow_migration/stage4.rs"]
pub mod v4_kvstore_converted;

// =============================================================================
// Stage 5: Full migration complete
// =============================================================================
#[path = "anyhow_migration/stage5.rs"]
pub mod v5_trait_converted;

// =============================================================================
// Main - run all stages
// =============================================================================

fn main() {
    println!("Anyhow to Rootcause Migration Example");
    println!("======================================\n");

    println!("=== Stage 1: Original anyhow code ===\n");
    if let Err(e) = v1_original_anyhow::main::run() {
        eprintln!("Error: {:#}\n", e);
    }

    println!("\n=== Stage 2: Application converted (top-down) ===\n");
    if let Err(e) = v2_main_converted::main::run() {
        eprintln!("Error: {}\n", e);
    }

    println!("\n=== Stage 3: Metrics library converted internally ===\n");
    if let Err(e) = v3_metrics_converted::main::run() {
        eprintln!("Error: {}\n", e);
    }

    println!("\n=== Stage 4: KV store converted internally ===\n");
    if let Err(e) = v4_kvstore_converted::main::run() {
        eprintln!("Error: {}\n", e);
    }

    println!("\n=== Stage 5: Full conversion (breaking change) ===\n");
    if let Err(e) = v5_trait_converted::main::run() {
        eprintln!("Error: {}\n", e);
    }

    println!("\n=== Migration complete! ===");
    println!("\nKey takeaways:");
    println!("  • Start top-down for immediate benefits");
    println!("  • Use .into_rootcause() when calling anyhow dependencies");
    println!("  • Use .into_anyhow() when exposing anyhow-compatible APIs");
    println!("  • Convert public interfaces last (breaking change)");
    println!("  • Migration can be gradual - you don't have to do it all at once!");
}
