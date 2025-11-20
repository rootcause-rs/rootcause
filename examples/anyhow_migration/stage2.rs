//! Stage 2: Application converted to rootcause
//!
//! Your application now uses rootcause, while dependencies still use anyhow.
//! This is the easiest first step - you get immediate benefits from rootcause
//! features like `.attach()` without touching any dependencies.
//!
//! **What changed:**
//! - Application's `main` module now returns `Result<(), Report>`
//! - Use `.into_rootcause()` when calling dependency methods
//! - Can now use `.attach()`, rich context, and other rootcause features
//!
//! **What stayed the same:**
//! - Metrics and KV store crates unchanged (still use anyhow)
//! - No breaking changes to any APIs
pub mod metrics {
    use std::collections::HashMap;

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

        fn flush(&mut self) -> anyhow::Result<()> {
            for (metric, value) in &self.metrics {
                self.store_metric(metric, *value)?;
            }
            self.metrics.clear();
            self.operations_since_flush = 0;
            Ok(())
        }

        fn store_metric(&self, metric: &str, _value: u64) -> anyhow::Result<()> {
            // Simulate external storage that can fail
            if metric.contains("error") {
                anyhow::bail!("Metrics storage service unavailable");
            }
            // In real code: send to Prometheus, StatsD, etc.
            Ok(())
        }
    }

    impl Metrics for MetricsCollector {
        fn record(&mut self, metric: &str, value: u64) -> anyhow::Result<()> {
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
// KV store crate - simulates another dependency
// -------------------------------------------------------------------------

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

// -------------------------------------------------------------------------
// Your application crate - NOW USING ROOTCAUSE!
// -------------------------------------------------------------------------

pub mod main {
    use rootcause::prelude::*;

    use super::{kv_store::KVStore, metrics::MetricsCollector};

    pub fn run() -> Result<(), Report> {
        println!("Running v2: Application converted to rootcause");

        let metrics = MetricsCollector::new(5);
        let mut store = KVStore::new(metrics);

        // Use .into_rootcause() to convert dependency results
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

        // Retrieve with conversion
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
