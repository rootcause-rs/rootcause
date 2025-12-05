//! Stage 3: Metrics library converted internally
//!
//! The metrics crate now uses rootcause internally while keeping its anyhow
//! API. This shows how library authors can adopt rootcause without breaking
//! users.
//!
//! **What changed:**
//! - Internal methods like `flush_internal()` now use rootcause
//! - Can add rich context (`.attach()`) in internal implementation
//! - Use `.into_anyhow()` to convert back when returning from trait methods
//!
//! **What stayed the same:**
//! - Public `Metrics` trait still uses `anyhow::Result` (no breaking change)
//! - KV store and application unchanged from stage 2

// -------------------------------------------------------------------------
// Metrics crate - CONVERTED INTERNALLY
// -------------------------------------------------------------------------

pub mod metrics {
    use std::collections::HashMap;

    use rootcause::{compat::anyhow1::IntoAnyhow, prelude::*};

    // Trait still uses anyhow - this is a public API, can't break it yet
    /// Trait for collecting metrics
    pub trait Metrics {
        fn record(&mut self, metric: &str, value: u64) -> anyhow::Result<()>;
    }

    /// A metrics collector that buffers and periodically flushes to storage
    pub struct MetricsCollector {
        metrics: HashMap<String, u64>,
        operations_since_flush: usize,
        flush_interval: usize,
    }

    impl MetricsCollector {
        pub fn new(flush_interval: usize) -> Self {
            Self {
                metrics: HashMap::new(),
                operations_since_flush: 0,
                flush_interval,
            }
        }

        // Internal method now uses rootcause
        fn flush_internal(&mut self) -> Result<(), Report> {
            for (metric, value) in &self.metrics {
                self.store_metric_internal(metric, *value)?;
            }
            self.metrics.clear();
            self.operations_since_flush = 0;
            Ok(())
        }

        // Internal method uses rootcause with rich context
        fn store_metric_internal(&self, metric: &str, value: u64) -> Result<(), Report> {
            if metric.contains("error") {
                return Err(report!("Metrics storage service unavailable")
                    .attach(format!("metric: {}", metric))
                    .attach(format!("value: {}", value)));
            }
            Ok(())
        }
    }

    impl Metrics for MetricsCollector {
        fn record(&mut self, metric: &str, value: u64) -> anyhow::Result<()> {
            // Use internal rootcause methods, convert back for trait
            *self.metrics.entry(metric.to_string()).or_insert(0) += value;
            self.operations_since_flush += 1;

            if self.operations_since_flush >= self.flush_interval {
                self.flush_internal().into_anyhow()?;
            }
            Ok(())
        }
    }
}

// KV store still unchanged from v1
pub mod kv_store {
    use std::collections::HashMap;

    use super::metrics::Metrics;

    /// In-memory key-value store with metrics
    pub struct KVStore<M: Metrics> {
        data: HashMap<String, String>,
        metrics: M,
    }

    impl<M: Metrics> KVStore<M> {
        pub fn new(metrics: M) -> Self {
            Self {
                data: HashMap::new(),
                metrics,
            }
        }

        pub fn get(&mut self, key: &str) -> anyhow::Result<Option<String>> {
            self.metrics.record("kv.get", 1)?;

            match self.data.get(key) {
                Some(value) => {
                    self.metrics.record("kv.hit", 1)?;
                    Ok(Some(value.clone()))
                }
                None => {
                    self.metrics.record("kv.miss", 1)?;
                    Ok(None)
                }
            }
        }

        pub fn set(&mut self, key: String, value: String) -> anyhow::Result<()> {
            if key.is_empty() {
                anyhow::bail!("Key cannot be empty");
            }
            self.metrics.record("kv.set", 1)?;
            self.data.insert(key, value);
            Ok(())
        }

        pub fn delete(&mut self, key: &str) -> anyhow::Result<()> {
            self.metrics.record("kv.delete", 1)?;
            self.data.remove(key);
            Ok(())
        }
    }
}

// Application crate unchanged from v2
pub mod main {
    use rootcause::prelude::*;

    use super::{kv_store::KVStore, metrics::MetricsCollector};

    pub fn run() -> Result<(), Report> {
        println!("Running v3: Metrics crate converted internally");

        let metrics = MetricsCollector::new(5);
        let mut store = KVStore::new(metrics);

        store
            .set("user:123".to_string(), "Alice".to_string())
            .into_rootcause()
            .context("Failed to store user")
            .attach("user_id: 123")?;

        store
            .set("user:456".to_string(), "Bob".to_string())
            .into_rootcause()
            .context("Failed to store user")
            .attach("user_id: 456")?;

        let name = store
            .get("user:123")
            .into_rootcause()
            .context("Failed to retrieve user")
            .attach("user_id: 123")?
            .ok_or_else(|| report!("User not found").attach("user_id: 123"))?;

        println!("  Retrieved user: {}", name);

        // Try to get non-existent key
        match store.get("user:999").into_rootcause()? {
            Some(name) => println!("  Found: {}", name),
            None => println!("  User 999 not found"),
        }

        Ok(())
    }
}
