//! Demonstrates using thiserror-generated errors with rootcause.
//!
//! Shows how to use thiserror errors as Report contexts, pattern matching,
//! and why rootcause's `.context()` provides better error chains than
//! thiserror's `#[from]`/`#[source]` nesting.

use std::{io, num::ParseIntError};

use rootcause::prelude::*;
use thiserror::Error;

mod example1 {
    use super::*;

    /// Pattern matching: Use Report<E> to preserve type information for
    /// conditional error handling based on the specific variant.
    #[derive(Error, Debug)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum ConfigError {
        #[error("Configuration file not found: {path}")]
        NotFound { path: String },

        #[error("Invalid configuration format")]
        InvalidFormat,

        #[error("Missing required field: {field}")]
        MissingField { field: String },
    }

    pub fn load_config(path: &str) -> Result<String, Report<ConfigError>> {
        if path.is_empty() {
            return Err(report!(ConfigError::InvalidFormat));
        }

        Err(report!(ConfigError::NotFound {
            path: path.to_string(),
        })
        .attach("Config version: 2.0"))
    }

    /// Demonstrates pattern matching on the error variant to decide recovery
    /// strategy.
    pub fn load_config_with_fallback(path: &str) -> Result<String, Report> {
        let report = match load_config(path) {
            Ok(config) => return Ok(config),
            Err(report) => report,
        };

        // Use matches!() to check for specific variant
        let should_fallback = matches!(report.current_context(), ConfigError::NotFound { .. });

        if should_fallback {
            println!("  Trying fallback config...");
            return Ok(load_config("/etc/app/config.toml")?);
        }

        // Use match to provide variant-specific context
        let context_msg = match report.current_context() {
            ConfigError::MissingField { field } => format!("Cannot proceed without '{field}'"),
            ConfigError::InvalidFormat => "Config file is corrupted".to_string(),
            ConfigError::NotFound { .. } => unreachable!(),
        };

        Err(report.context(context_msg))?
    }
}

mod example2 {
    use super::*;

    /// Easiest migration: Keep thiserror's #[from] nesting, wrap in Report.
    /// This works well for existing codebases using thiserror, but only
    /// tracks a single location per error.
    #[derive(Error, Debug)]
    pub enum AppError {
        #[error("Database error")]
        Database(#[from] DatabaseError),

        #[error("Configuration error")]
        Config(#[from] example1::ConfigError),

        #[error("I/O error: {0}")]
        Io(#[from] io::Error),

        #[error("Parse error: {0}")]
        Parse(#[from] ParseIntError),
    }

    #[expect(dead_code, reason = "example code: not all variants are used")]
    #[derive(Error, Debug)]
    pub enum DatabaseError {
        #[error("Connection failed: {reason}")]
        ConnectionFailed { reason: String },

        #[error("Query timeout after {seconds}s")]
        QueryTimeout { seconds: u64 },

        #[error("Constraint violation: {constraint}")]
        ConstraintViolation { constraint: String },
    }

    pub fn query_database(_id: u32) -> Result<String, DatabaseError> {
        Err(DatabaseError::QueryTimeout { seconds: 30 })
    }

    pub fn process_user_data(user_id: u32) -> Result<String, Report<AppError>> {
        // Use .map_err() to invoke thiserror's #[from] conversions
        let data = query_database(user_id).map_err(AppError::from)?;

        Ok(data)
    }

    pub fn process_user_with_context(user_id: u32) -> Result<String, Report<AppError>> {
        let data = process_user_data(user_id).attach(format!("Processing user: {user_id}"))?;

        Ok(data)
    }
}

mod example3 {
    use super::*;

    /// Best for new code: Flat thiserror enums with rootcause nesting.
    /// Compare to example2 - same logic, but tracks multiple locations
    /// in the error chain for better debugging.
    #[derive(Error, Debug)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum AppError {
        #[error("Database operation failed")]
        Database,

        #[error("Configuration operation failed")]
        Config,

        #[error("I/O operation failed")]
        Io,

        #[error("Parse operation failed")]
        Parse,
    }

    /// Detailed errors go in child reports, not nested in AppError.
    #[derive(Error, Debug, Clone)]
    #[expect(dead_code, reason = "demonstrates database error variants")]
    pub enum DatabaseError {
        #[error("Connection failed: {reason}")]
        ConnectionFailed { reason: String },

        #[error("Query timeout after {seconds}s")]
        QueryTimeout { seconds: u64 },

        #[error("Constraint violation: {constraint}")]
        ConstraintViolation { constraint: String },
    }

    pub fn query_database(_id: u32) -> Result<String, Report<DatabaseError>> {
        Err(report!(DatabaseError::QueryTimeout { seconds: 30 }))
    }

    pub fn process_user_data(user_id: u32) -> Result<String, Report> {
        // Add flat enum variant as parent context via .context()
        let data = query_database(user_id).context(AppError::Database)?;

        Ok(data)
    }

    pub fn process_user_with_context(user_id: u32) -> Result<String, Report> {
        let data = process_user_data(user_id).attach(format!("Processing user: {user_id}"))?;

        Ok(data)
    }
}

fn main() {
    println!("=== Example 1: Basic thiserror integration ===\n");

    if let Err(e) = example1::load_config("") {
        println!("Direct thiserror error:\n{e}\n");
    }

    if let Err(e) = example1::load_config_with_fallback("/nonexistent/config.toml") {
        println!("Pattern matching on thiserror error:\n{e}\n");
    }

    println!("=== Example 2 vs 3: Comparison ===");
    println!("Both examples do the same thing, but example2 uses thiserror nesting");
    println!("while example3 uses rootcause nesting.\n");

    println!("Example 2 (thiserror #[from] - easier migration):\n");

    if let Err(e) = example2::process_user_data(123) {
        println!("{e}\n");
    }

    if let Err(e) = example2::process_user_with_context(123) {
        println!("{e}\n");
    }

    println!("Example 3 (rootcause .context() - better debugging):\n");

    if let Err(e) = example3::process_user_data(123) {
        println!("{e}\n");
    }

    if let Err(e) = example3::process_user_with_context(123) {
        println!("{e}\n");
    }
}
