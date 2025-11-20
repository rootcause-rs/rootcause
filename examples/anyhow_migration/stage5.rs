//! Stage 5: Full migration complete
//!
//! Everything now uses rootcause - the migration is complete!
//! This stage shows the final, clean state with no conversions needed.
//!
//! **What changed:**
//! - `Metrics` trait now uses `Result<(), Report>` (**breaking change**)
//! - All `.into_rootcause()` and `.into_anyhow()` calls removed
//! - Clean, consistent rootcause usage throughout
//!
//! **Benefits:**
//! - No conversion overhead
//! - Consistent error handling patterns across all crates
//! - Full access to rootcause features everywhere

// -------------------------------------------------------------------------
// Metrics crate - TRAIT CONVERTED
// -------------------------------------------------------------------------

pub mod metrics {
    use std::collections::HashMap;

    use rootcause::prelude::*;

    // Trait now uses rootcause - this is a breaking change
    /// Trait for collecting metrics
    pub trait Metrics {
        fn record(&mut self, metric: &str, value: u64) -> Result<(), Report>;
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

        fn flush(&mut self) -> Result<(), Report> {
            for (metric, value) in &self.metrics {
                self.store_metric(metric, *value)?;
            }
            self.metrics.clear();
            self.operations_since_flush = 0;
            Ok(())
        }

        fn store_metric(&self, metric: &str, value: u64) -> Result<(), Report> {
            if metric.contains("error") {
                return Err(report!("Metrics storage service unavailable")
                    .attach(format!("metric: {}", metric))
                    .attach(format!("value: {}", value)));
            }
            Ok(())
        }
    }

    impl Metrics for MetricsCollector {
        fn record(&mut self, metric: &str, value: u64) -> Result<(), Report> {
            *self.metrics.entry(metric.to_string()).or_insert(0) += value;
            self.operations_since_flush += 1;

            if self.operations_since_flush >= self.flush_interval {
                self.flush()?;
            }
            Ok(())
        }
    }
}

// -------------------------------------------------------------------------
// KV store - CLEAN, using rootcause throughout
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

        // Everything now uses rootcause - clean and simple!
        pub fn get(&mut self, key: &str) -> Result<Option<String>, Report> {
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

        pub fn set(&mut self, key: String, value: String) -> Result<(), Report> {
            if key.is_empty() {
                return Err(report!("Key cannot be empty")
                    .attach(format!("attempted key: '{}'", key))
                    .attach(format!("value length: {}", value.len())));
            }

            self.metrics.record("kv.set", 1)?;
            self.data.insert(key, value);
            Ok(())
        }

        pub fn delete(&mut self, key: &str) -> Result<(), Report> {
            self.metrics.record("kv.delete", 1)?;
            self.data.remove(key);
            Ok(())
        }
    }
}

// -------------------------------------------------------------------------
// Application crate - CLEAN, no conversions needed
// -------------------------------------------------------------------------

pub mod main {
    use rootcause::prelude::*;

    use super::{kv_store::KVStore, metrics::MetricsCollector};

    pub fn run() -> Result<(), Report> {
        println!("Running v5: Full conversion - migration complete!");

        let metrics = MetricsCollector::new(5);
        let mut store = KVStore::new(metrics);

        // Clean rootcause code throughout - no conversions!
        store
            .set("user:123".to_string(), "Alice".to_string())
            .context("Failed to store user")
            .attach("user_id: 123")?;

        store
            .set("user:456".to_string(), "Bob".to_string())
            .context("Failed to store user")
            .attach("user_id: 456")?;

        let name = store
            .get("user:123")
            .context("Failed to retrieve user")
            .attach("user_id: 123")?
            .ok_or_else(|| report!("User not found").attach("user_id: 123"))?;

        println!("  Retrieved user: {}", name);

        match store.get("user:999")? {
            Some(name) => println!("  Found: {}", name),
            None => println!("  User 999 not found"),
        }

        Ok(())
    }
}
