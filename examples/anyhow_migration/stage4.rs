//! Stage 4: KV store converted internally
//!
//! The KV store now uses rootcause internally while keeping its anyhow API.
//! This stage demonstrates managing conversions in both directions.
//!
//! **What changed:**
//! - KV store methods split into public (anyhow) and internal (rootcause)
//!   versions
//! - Use `.into_rootcause()` when calling metrics (still anyhow)
//! - Use `.into_anyhow()` when returning from public methods
//! - Rich error context with `.attach()` in internal methods
//!
//! **What stayed the same:**
//! - Public KV store API still uses `anyhow::Result` (no breaking change)
//! - Metrics and application unchanged from stage 3
pub mod metrics {
    use std::collections::HashMap;

    use rootcause::prelude::*;

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

        fn flush_internal(&mut self) -> Result<(), Report> {
            for (metric, value) in &self.metrics {
                self.store_metric_internal(metric, *value)?;
            }
            self.metrics.clear();
            self.operations_since_flush = 0;
            Ok(())
        }

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
            *self.metrics.entry(metric.to_string()).or_insert(0) += value;
            self.operations_since_flush += 1;

            if self.operations_since_flush >= self.flush_interval {
                self.flush_internal().into_anyhow()?;
            }
            Ok(())
        }
    }
}

// -------------------------------------------------------------------------
// KV store - CONVERTED INTERNALLY
// -------------------------------------------------------------------------

pub mod kv_store {
    use std::collections::HashMap;

    use rootcause::prelude::*;

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

        // Public API still uses anyhow
        pub fn get(&mut self, key: &str) -> anyhow::Result<Option<String>> {
            self.get_internal(key).into_anyhow()
        }

        // Internal implementation uses rootcause
        fn get_internal(&mut self, key: &str) -> Result<Option<String>, Report> {
            // Metrics trait still uses anyhow, need to convert
            self.metrics.record("kv.get", 1).into_rootcause()?;

            match self.data.get(key) {
                Some(value) => {
                    self.metrics.record("kv.hit", 1).into_rootcause()?;
                    Ok(Some(value.clone()))
                }
                None => {
                    self.metrics.record("kv.miss", 1).into_rootcause()?;
                    Ok(None)
                }
            }
        }

        pub fn set(&mut self, key: String, value: String) -> anyhow::Result<()> {
            self.set_internal(key, value).into_anyhow()
        }

        fn set_internal(&mut self, key: String, value: String) -> Result<(), Report> {
            if key.is_empty() {
                return Err(report!("Key cannot be empty")
                    .attach(format!("attempted key: '{}'", key))
                    .attach(format!("value length: {}", value.len())));
            }

            self.metrics.record("kv.set", 1).into_rootcause()?;
            self.data.insert(key, value);
            Ok(())
        }

        pub fn delete(&mut self, key: &str) -> anyhow::Result<()> {
            self.delete_internal(key).into_anyhow()
        }

        fn delete_internal(&mut self, key: &str) -> Result<(), Report> {
            self.metrics.record("kv.delete", 1).into_rootcause()?;
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
        println!("Running v4: KV store converted internally");

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
