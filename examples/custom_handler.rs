//! Demonstrates creating custom AttachmentHandler implementations.
//!
//! This example shows:
//! 1. Implementing AttachmentHandler for specialized formatting
//! 2. Using attach_custom() with custom handlers
//! 3. Different handler strategies (hexdump, table, JSON-like)
//! 4. When to use custom handlers vs Display/Debug

use rootcause::{handlers::AttachmentHandler, prelude::*};
use std::io;

// ============================================================================
// Example 1: Hexdump handler for binary data
// ============================================================================

/// Binary data that we want to display as a hexdump.
struct BinaryData(Vec<u8>);

impl core::fmt::Display for BinaryData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} bytes of binary data", self.0.len())
    }
}

impl core::fmt::Debug for BinaryData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BinaryData({:?})", self.0)
    }
}

/// Custom handler that formats binary data as a hexdump.
struct Hexdump;

impl AttachmentHandler<BinaryData> for Hexdump {
    fn display(attachment: &BinaryData, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Hexdump ({} bytes):", attachment.0.len())?;
        for (i, chunk) in attachment.0.chunks(16).enumerate() {
            write!(f, "{:04x}: ", i * 16)?;
            for byte in chunk {
                write!(f, "{:02x} ", byte)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    fn debug(attachment: &BinaryData, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(attachment, f)
    }
}

fn parse_binary_message() -> Result<String, Report> {
    let corrupt_data = BinaryData(vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);

    Err(report!(io::Error::new(
        io::ErrorKind::InvalidData,
        "Corrupt message"
    ))
    .attach_custom::<Hexdump, _>(corrupt_data)
    .into_dyn_any())
}

// ============================================================================
// Example 2: Table handler for structured data
// ============================================================================

/// Statistics that we want to display as a table.
struct Statistics {
    requests: u64,
    errors: u64,
    avg_latency_ms: f64,
}

impl core::fmt::Display for Statistics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "requests={}, errors={}, latency={}ms",
            self.requests, self.errors, self.avg_latency_ms
        )
    }
}

impl core::fmt::Debug for Statistics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Statistics")
            .field("requests", &self.requests)
            .field("errors", &self.errors)
            .field("avg_latency_ms", &self.avg_latency_ms)
            .finish()
    }
}

/// Custom handler that formats statistics as a nice table.
struct TableFormat;

impl AttachmentHandler<Statistics> for TableFormat {
    fn display(stats: &Statistics, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "┌─────────────────┬──────────┐")?;
        writeln!(f, "│ Metric          │ Value    │")?;
        writeln!(f, "├─────────────────┼──────────┤")?;
        writeln!(f, "│ Requests        │ {:>8} │", stats.requests)?;
        writeln!(f, "│ Errors          │ {:>8} │", stats.errors)?;
        writeln!(f, "│ Avg Latency (ms)│ {:>8.2} │", stats.avg_latency_ms)?;
        write!(f, "└─────────────────┴──────────┘")
    }

    fn debug(stats: &Statistics, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(stats, f)
    }
}

fn report_server_overload() -> Result<(), Report> {
    let stats = Statistics {
        requests: 1_000_000,
        errors: 150_000,
        avg_latency_ms: 2543.7,
    };

    Err(report!("Server overload detected")
        .attach_custom::<TableFormat, _>(stats)
        .into_dyn_any())
}

// ============================================================================
// Example 3: Compact JSON-like handler
// ============================================================================

/// Configuration that we want to show in a compact format.
struct Config {
    host: String,
    port: u16,
    timeout_ms: u64,
    retry_count: u32,
}

impl core::fmt::Display for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl core::fmt::Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Config")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("timeout_ms", &self.timeout_ms)
            .field("retry_count", &self.retry_count)
            .finish()
    }
}

/// Custom handler that formats config as compact JSON-like output.
struct CompactJson;

impl AttachmentHandler<Config> for CompactJson {
    fn display(config: &Config, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{{ host: \"{}\", port: {}, timeout_ms: {}, retry_count: {} }}",
            config.host, config.port, config.timeout_ms, config.retry_count
        )
    }

    fn debug(config: &Config, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(config, f)
    }
}

fn connect_to_server() -> Result<(), Report> {
    let config = Config {
        host: "api.example.com".to_string(),
        port: 8443,
        timeout_ms: 5000,
        retry_count: 3,
    };

    Err(report!(io::Error::new(
        io::ErrorKind::TimedOut,
        "Connection timeout"
    ))
    .attach("Using custom handler for config:")
    .attach_custom::<CompactJson, _>(config)
    .into_dyn_any())
}

// ============================================================================
// Example 4: Comparison - same data with different handlers
// ============================================================================

fn demonstrate_handler_comparison() -> Result<(), Report> {
    let stats = Statistics {
        requests: 5000,
        errors: 42,
        avg_latency_ms: 123.45,
    };

    Err(report!("Handler comparison")
        .attach("With Display handler:")
        .attach(format!(
            "requests={}, errors={}, latency={}ms",
            stats.requests, stats.errors, stats.avg_latency_ms
        ))
        .attach("With TableFormat handler:")
        .attach_custom::<TableFormat, _>(stats)
        .into_dyn_any())
}

fn main() {
    println!("=== Example 1: Hexdump handler for binary data ===\n");
    match parse_binary_message() {
        Ok(_) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 2: Table handler for statistics ===\n");
    match report_server_overload() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 3: Compact JSON-like handler ===\n");
    match connect_to_server() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("=== Example 4: Handler comparison ===\n");
    match demonstrate_handler_comparison() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}"),
    }
}
