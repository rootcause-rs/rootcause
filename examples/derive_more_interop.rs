//! Using derive_more errors with rootcause: choosing your structure.
//!
//! This example demonstrates four structural approaches for integrating
//! derive_more-generated error types with rootcause Reports, showing the
//! trade-offs between migration effort, type safety, and debuggability.
//!
//! # The Structural Spectrum
//!
//! When using derive_more with rootcause, you can choose from four patterns
//! that represent different points on the migration and design spectrum:
//!
//! 1. **Type-nested hierarchy** - `Error -> AppError(Error)` via `#[from]`
//!    - Best for: Initial migration with minimal code changes
//!    - Trade-off: Only one location captured per error
//!
//! 2. **Early Report creation** - `Report<Error> -> Report<AppError>` via
//!    `context_transform`
//!    - Best for: Multiple locations captured while preserving type-level
//!      nesting
//!    - Trade-off: Must match error type hierarchy, choose between losing
//!      location info or performance cost
//!
//! 3. **Flat enums with Report nesting** - Category markers with child Reports
//!    - Best for: Flexible categorization independent of error type structure
//!    - Trade-off: Enum granularity must match your needs or you'll use
//!      iter_reports
//!
//! 4. **Dynamic propagation with selective handling** - `Report<Dynamic>`
//!    - Best for: Need `.attach()` but only handle some error variants
//!    - Trade-off: Lose type information, requires downcasting
//!
//! **These styles can coexist** in the same codebase! A function returning
//! `Result<_, Report<DatabaseError>>` can be called by code using any style.
//! Each style just wraps/converts the Report differently. This makes gradual
//! migration practical.
//!
//! # What This Example Teaches
//!
//! - How error structure affects migration effort and pattern matching
//!   convenience
//! - The difference between type-level nesting (styles 1-2) vs Report-level
//!   nesting (style 3)
//! - When early Report creation captures more locations (style 2)
//! - When flexible categorization matters more than rigid type hierarchies
//!   (style 3)
//! - The trade-off between enum granularity and pattern matching complexity
//! - How to mix styles during gradual migration
//!
//! For a comparison of context transformation methods, see
//! [`context_methods.rs`](context_methods.rs).

use std::io;

use derive_more::{Display, Error, From};
use rootcause::{ReportConversion, prelude::*};

// ============================================================================
// Style 1: Type-nested hierarchy (pure derive_more + Report wrapper)
// ============================================================================
//
// Minimal migration path: keep your existing derive_more error types and
// `From` conversions, just wrap the Result in Report at API boundaries.
// Error nesting happens at the TYPE level (one error inside another enum),
// so only one location is captured per error chain.
//
// Migration effort: LOW - Change `Result<T, E>` to `Result<T, Report<E>>`
// Location tracking: MINIMAL - One location per error
// When to use: Initial adoption, preserving existing error hierarchies

mod type_nested {
    use super::*;

    /// Top-level error with nested domain errors using derive_more's `From`.
    ///
    /// The key characteristic: errors nest at the TYPE level (DatabaseError
    /// inside AppError), not at the Report level. This means `From`
    /// conversions happen before any Report is created.
    #[derive(Error, Debug, Display, From)]
    pub enum AppError {
        #[display("Database error")]
        Database(DatabaseError),

        #[display("Configuration error")]
        Config(ConfigError),

        #[display("I/O error: {_0}")]
        Io(io::Error),
    }

    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum DatabaseError {
        #[display("Connection failed: {reason}")]
        ConnectionFailed { reason: String },

        #[display("Query timeout after {seconds}s")]
        QueryTimeout { seconds: u64 },
    }

    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum ConfigError {
        #[display("Invalid format in {file}")]
        InvalidFormat { file: String },

        #[display("Missing field: {field}")]
        MissingField { field: String },
    }

    // --- The pattern: plain errors + From conversions ---

    /// Lower-level functions return plain derive_more errors (no Report).
    pub fn query_database(_id: u32) -> Result<String, DatabaseError> {
        Err(DatabaseError::QueryTimeout { seconds: 30 })
    }

    /// Higher-level functions convert via From and wrap in Report.
    /// Only ONE location is captured (here, where the Report is created).
    pub fn process_request(user_id: u32) -> Result<String, Report<AppError>> {
        // .map_err uses From to convert DatabaseError -> AppError
        // Then ? converts AppError -> Report<AppError>
        let data = query_database(user_id).map_err(AppError::from)?;
        Ok(data)
    }

    /// Demonstrates pattern matching with type-nested errors.
    /// Logs specific errors but propagates the full result.
    pub fn handle_error(user_id: u32) -> Result<String, Report<AppError>> {
        let result = process_request(user_id);

        // Log specific errors we care about
        if let Err(ref report) = result
            && let AppError::Database(DatabaseError::ConnectionFailed { reason }) =
                report.current_context()
        {
            eprintln!("[LOG] Database connection failed: {reason}");
        }

        result
    }
}

// ============================================================================
// Style 2: Early Report creation with type-level nesting
// ============================================================================
//
// Progressive migration: keep your derive_more error structure (including
// From), but return Reports from lower-level functions. Use
// context_transform to wrap errors at the TYPE level (DatabaseError inside
// AppError::Database variant).
//
// **Key point**: context_transform does NOT run hooks (no new location
// captured at wrapping site). It transforms the context value in-place. You
// get multiple locations by creating Reports earlier in the call stack (each
// call to report!() captures a location), not from the wrapping itself.
//
// Migration effort: MEDIUM - Change function signatures to return Report<E>
// Location tracking: GOOD - Locations captured where Reports are created
// When to use: Want more location tracking while preserving error type hierarchy

mod report_nested {
    use super::*;

    /// Same error definitions as style 1 - hierarchical with From.
    /// The difference: lower functions return Report<E>, not plain E.
    #[derive(Error, Debug, Display, From)]
    pub enum AppError {
        #[display("Database error")]
        Database(DatabaseError),

        #[display("Configuration error")]
        Config(ConfigError),

        #[display("I/O error: {_0}")]
        Io(io::Error),
    }

    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum DatabaseError {
        #[display("Connection failed: {reason}")]
        ConnectionFailed { reason: String },

        #[display("Query timeout after {seconds}s")]
        QueryTimeout { seconds: u64 },
    }

    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum ConfigError {
        #[display("Invalid format in {file}")]
        InvalidFormat { file: String },

        #[display("Missing field: {field}")]
        MissingField { field: String },
    }

    // --- Direct conversion approach ---

    /// Lower-level functions return Report<SpecificError> (not plain errors).
    pub fn direct_query_database(_id: u32) -> Result<String, Report<DatabaseError>> {
        Err(report!(DatabaseError::QueryTimeout { seconds: 30 }))
    }

    /// Higher-level functions use context_transform to change Report type.
    /// Location captured in direct_query_database (report!() call), but NOT here
    /// (context_transform doesn't run hooks).
    pub fn direct_process_request(user_id: u32) -> Result<String, Report<AppError>> {
        // context_transform works directly on Result via ResultExt
        // In-place type change: DatabaseError → AppError::Database(DatabaseError)
        // No new report node, no hooks run
        let data = direct_query_database(user_id).context_transform(AppError::Database)?;
        Ok(data)
    }

    // --- Systematic conversion approach ---

    impl<T> ReportConversion<DatabaseError, markers::Mutable, T> for AppError
    where
        AppError: markers::ObjectMarkerFor<T>,
    {
        fn convert_report(
            report: Report<DatabaseError, markers::Mutable, T>,
        ) -> Report<Self, markers::Mutable, T> {
            // In-place transformation: preserves structure, no hooks
            report.context_transform(AppError::Database)
        }
    }

    pub fn systematic_query_database(_id: u32) -> Result<String, Report<DatabaseError>> {
        Err(report!(DatabaseError::QueryTimeout { seconds: 30 }))
    }

    pub fn systematic_process_request(user_id: u32) -> Result<String, Report<AppError>> {
        // ReportConversion provides systematic type conversion
        let data = systematic_query_database(user_id).context_to::<AppError>()?;
        Ok(data)
    }

    /// Demonstrates pattern matching with report-nested errors.
    /// Logs specific errors but propagates the full result.
    pub fn handle_error(user_id: u32) -> Result<String, Report<AppError>> {
        let result = direct_process_request(user_id);

        // Log specific errors we care about
        if let Err(ref report) = result
            && let AppError::Database(DatabaseError::ConnectionFailed { reason }) =
                report.current_context()
        {
            eprintln!("[LOG] Database connection failed: {reason}");
        }

        result
    }
}

// ============================================================================
// Style 3: Flat enums with Report-level parent-child nesting
// ============================================================================
//
// Enum variants are simple category markers. Detailed errors are preserved as
// child Reports via .context(), creating parent-child Report chains. Like style
// 2, you implement ReportConversion once and use context_to() at call sites.
//
// **Key differences from Style 2:**
//
// **Style 2** (context_transform):
// - Type-level nesting: DatabaseError inside AppError::Database(DatabaseError)
// - NO hooks at wrapping site (context_transform doesn't run hooks)
// - Must match type structure 1-to-1 (due to `From` pattern)
// - Can pattern match directly: AppError::Database(DatabaseError::ConnectionFailed)
//
// **Style 3** (context):
// - Report-level nesting: DatabaseError as child Report under AppError category
// - RUNS hooks at wrapping site (context() captures fresh locations)
// - Flexible categorization: can split, merge, or remap error types
// - Pattern matching on root only: AppError::DatabaseConnectionFailed
//   (child details need iter_reports() + downcast)
//
// **Choose Style 3 when:**
// - Location tracking at conversion points matters (hooks at each wrapper)
// - Your categorization doesn't match error type structure 1-to-1
// - You want flexibility to split/merge/remap error categories
//
// **Choose Style 2 when:**
// - You want to preserve existing `From`-based type hierarchy
// - Direct pattern matching on nested types is important
// - Don't need hooks at conversion points (only at error creation)
//
// Migration effort: HIGH - Redesign error types (remove `From`, flatten
// hierarchy) Location tracking: EXCELLENT - Hooks run at every wrapping point
// When to use: Need flexible categorization or maximum location tracking

mod flat {
    use super::*;

    /// Flat enum demonstrating flexible categorization.
    ///
    /// This approach:
    /// - Creates parent-child Report chains with full location tracking
    ///   (.context() runs hooks)
    /// - Detailed errors stay independent as child Reports (not nested in
    ///   variants)
    /// - Flexible: categories don't need 1-to-1 mapping with underlying error
    ///   types
    ///
    /// **Examples of flexibility in this enum:**
    /// - **Split**: DatabaseError::ConnectionFailed gets its own top-level
    ///   variant
    /// - **Merge**: Both ConfigError and io::Error map to the same System
    ///   variant
    /// - **Context-dependent**: Same underlying error could map to different
    ///   categories based on where/how it occurs (not shown here, but possible)
    ///
    /// **Key advantage over Style 2**: Style 2's From requires 1-to-1
    /// mapping between your AppError variants and underlying error types.
    /// This style lets you design categories that match your domain needs,
    /// not your error type structure.
    #[derive(Error, Debug, Display)]
    pub enum AppError {
        #[display("Database connection failed")]
        DatabaseConnectionFailed,

        #[display("Database operation failed")]
        DatabaseOther,

        #[display("System configuration or I/O error")]
        System,
    }

    /// Detailed error types live independently, not nested in AppError.
    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum DatabaseError {
        #[display("Connection failed: {reason}")]
        ConnectionFailed { reason: String },

        #[display("Query timeout after {seconds}s")]
        QueryTimeout { seconds: u64 },
    }

    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum ConfigError {
        #[display("Invalid format in {file}")]
        InvalidFormat { file: String },

        #[display("Missing field: {field}")]
        MissingField { field: String },
    }

    // --- Direct conversion approach ---

    pub fn direct_query_database(_id: u32) -> Result<String, Report<DatabaseError>> {
        Err(report!(DatabaseError::QueryTimeout { seconds: 30 }))
    }

    pub fn direct_process_request(user_id: u32) -> Result<String, Report<AppError>> {
        // Manually categorize based on the specific error
        match direct_query_database(user_id) {
            Ok(d) => Ok(d),
            Err(report) => match report.current_context() {
                DatabaseError::ConnectionFailed { .. } => {
                    Err(report.context(AppError::DatabaseConnectionFailed))
                }
                DatabaseError::QueryTimeout { .. } => Err(report.context(AppError::DatabaseOther)),
            },
        }
    }

    // --- Systematic conversion approach ---

    impl<T> ReportConversion<DatabaseError, markers::Mutable, T> for AppError
    where
        AppError: markers::ObjectMarkerFor<T>,
    {
        fn convert_report(
            report: Report<DatabaseError, markers::Mutable, T>,
        ) -> Report<Self, markers::Mutable, T> {
            // Map to fine-grained category based on the specific error
            if matches!(
                report.current_context(),
                DatabaseError::ConnectionFailed { .. }
            ) {
                report.context(Self::DatabaseConnectionFailed)
            } else {
                report.context(Self::DatabaseOther)
            }
        }
    }

    // Both ConfigError and io::Error map to the same System category
    impl<T> ReportConversion<ConfigError, markers::Mutable, T> for AppError
    where
        AppError: markers::ObjectMarkerFor<T>,
    {
        fn convert_report(
            report: Report<ConfigError, markers::Mutable, T>,
        ) -> Report<Self, markers::Mutable, T> {
            report.context(AppError::System)
        }
    }

    impl<T> ReportConversion<io::Error, markers::Mutable, T> for AppError
    where
        AppError: markers::ObjectMarkerFor<T>,
    {
        fn convert_report(
            report: Report<io::Error, markers::Mutable, T>,
        ) -> Report<Self, markers::Mutable, T> {
            report.context(AppError::System)
        }
    }

    pub fn systematic_query_database(_id: u32) -> Result<String, Report<DatabaseError>> {
        Err(report!(DatabaseError::QueryTimeout { seconds: 30 }))
    }

    pub fn systematic_process_request(user_id: u32) -> Result<String, Report<AppError>> {
        // ReportConversion handles nesting systematically
        let data = systematic_query_database(user_id).context_to::<AppError>()?;
        Ok(data)
    }

    /// Demonstrates pattern matching with fine-grained flat errors.
    /// With fine-grained categories, we can match directly on AppError
    /// variants.
    pub fn handle_error(user_id: u32) -> Result<String, Report<AppError>> {
        let result = systematic_process_request(user_id);

        // Direct pattern matching on fine-grained category
        if let Err(ref report) = result
            && matches!(report.current_context(), AppError::DatabaseConnectionFailed)
        {
            eprintln!("[LOG] Database connection failed");
        }

        result
    }

    // --- Alternative: Even coarser categories ---
    //
    // You could use even coarser categories. For example, a single `Database`
    // variant instead of splitting ConnectionFailed. This is simpler to
    // maintain but requires iter_reports() + downcast to match on specific
    // errors:
    //
    // ```
    // pub enum AppErrorCoarse {
    //     #[error("Database operation failed")]
    //     Database,
    //     // ... other coarse categories
    // }
    //
    // // Pattern matching requires iteration:
    // if let Err(ref report) = result
    //     && matches!(report.current_context(), AppErrorCoarse::Database)
    // {
    //     for child in report.iter_reports() {
    //         if let Some(DatabaseError::ConnectionFailed { reason }) =
    //             child.downcast_current_context()
    //         {
    //             eprintln!("[LOG] Connection failed: {reason}");
    //             break;
    //         }
    //     }
    // }
    // ```
    //
    // Choose granularity based on which errors you need to handle
    // programmatically.
}

// ============================================================================
// Style 4: Dynamic propagation with selective handling
// ============================================================================
//
// You have specific error types but don't want a wrapper enum. Functions
// return Report<SpecificError>, but callers propagate as Report (dynamic)
// and downcast only for errors they need to handle programmatically.
//
// **When to use this pattern:**
// Convenience matters more than type safety. You want the flexibility to
// propagate errors dynamically (no wrapper enum needed) while selectively
// handling only specific variants that matter. The key benefit: you can use
// `.attach()` to add context WITHOUT changing the root error type, making
// the code feel lightweight while still being inspectable.
//
// **Trade-offs:**
// - Pros: Lightweight propagation, selective handling, rich context via
//   .attach()
// - Cons: Lose type safety at boundaries, downcasting is more awkward than
//   pattern matching, no compiler help ensuring you handle what you need
//
// Migration effort: LOW - Dynamic propagation with typed lower functions
// Location tracking: GOOD - Reports track locations
// When to use: Convenience over type safety, selective handling without wrapper
// enum

mod dynamic_propagation {
    use super::*;

    /// Specific error types from domain operations.
    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum DatabaseError {
        #[display("Connection failed: {reason}")]
        ConnectionFailed { reason: String },

        #[display("Query timeout after {seconds}s")]
        QueryTimeout { seconds: u64 },

        #[display("Record not found: {id}")]
        NotFound { id: u32 },
    }

    #[derive(Error, Debug, Display)]
    #[expect(dead_code, reason = "example code: not all variants are used")]
    pub enum ConfigError {
        #[display("Invalid format in {file}")]
        InvalidFormat { file: String },

        #[display("Missing field: {field}")]
        MissingField { field: String },
    }

    // --- The pattern: specific types + dynamic propagation ---

    /// Low-level functions return specific error types.
    pub fn query_database(id: u32) -> Result<String, Report<DatabaseError>> {
        if id == 404 {
            Err(report!(DatabaseError::NotFound { id }))
        } else {
            Err(report!(DatabaseError::QueryTimeout { seconds: 30 }))
        }
    }

    #[expect(dead_code, reason = "example code: illustrates the pattern")]
    pub fn load_config(_path: &str) -> Result<String, Report<ConfigError>> {
        Err(report!(ConfigError::InvalidFormat {
            file: "app.toml".to_string()
        }))
    }

    /// Higher-level function propagates dynamically but handles specific
    /// errors.
    ///
    /// This is the key pattern: selectively handle errors you care about
    /// (like NotFound) while propagating everything else dynamically. The
    /// convenience comes from adding rich context via .attach() without
    /// changing the root error type—keeping the code lightweight.
    pub fn process_request(user_id: u32, _config_path: &str) -> Result<String, Report> {
        // Selectively handle NotFound, propagate everything else
        match query_database(user_id).attach("Querying database") {
            Ok(d) => Ok(d),
            Err(report) => {
                // report is still Report<DatabaseError> here, so direct pattern matching works
                if let DatabaseError::NotFound { id } = report.current_context() {
                    println!("  Record {id} not found, using default");
                    Ok("default".to_string())
                } else {
                    // .into() converts Report<DatabaseError> -> Report (dynamic)
                    Err(report.into())
                }
            }
        }
    }

    /// Demonstrates downcasting with dynamic reports.
    /// Logs specific errors but propagates the full result.
    pub fn handle_error(user_id: u32) -> Result<String, Report> {
        let result = process_request(user_id, "config.toml");

        // Log specific errors via downcasting
        if let Err(ref report) = result
            && let Some(DatabaseError::ConnectionFailed { reason }) =
                report.downcast_current_context()
        {
            eprintln!("[LOG] Database connection failed: {reason}");
        }

        result
    }
}

fn main() {
    println!("=== Style 1: Type-Nested Hierarchy ===");
    println!("Minimal migration: use `From` to nest errors at type level.\n");

    if let Err(e) = type_nested::process_request(123) {
        println!("{e}\n");
    }

    println!("=== Style 2: Early Report Creation ===");
    println!("Return Reports from lower functions, but still nest at TYPE level.\n");

    println!("Direct conversion (context_transform):");
    if let Err(e) = report_nested::direct_process_request(123) {
        println!("{e}\n");
    }

    println!("Systematic conversion (ReportConversion):");
    if let Err(e) = report_nested::systematic_process_request(123) {
        println!("{e}\n");
    }

    println!("=== Style 3: Flat Enums with Report Nesting ===");
    println!("Category markers with parent-child Report chains (not type-level nesting).\n");

    println!("Direct conversion (.context):");
    if let Err(e) = flat::direct_process_request(123) {
        println!("{e}\n");
    }

    println!("Systematic conversion (ReportConversion):");
    if let Err(e) = flat::systematic_process_request(123) {
        println!("{e}\n");
    }

    println!("=== Style 4: Dynamic Propagation with Selective Handling ===");
    println!("Return specific types but propagate dynamically, handle via downcasting.\n");

    println!("Timeout error (propagates):");
    if let Err(e) = dynamic_propagation::process_request(123, "config.toml") {
        println!("{e}\n");
    }

    println!("NotFound error (handled selectively):");
    if let Err(e) = dynamic_propagation::process_request(404, "config.toml") {
        println!("{e}\n");
    } else {
        println!("  (NotFound was handled, returned Ok)\n");
    }

    println!("=== Error Handling Patterns ===");
    println!("Each style logs specific errors then propagates the result.\n");

    println!("Type-nested:");
    match type_nested::handle_error(123) {
        Ok(data) => println!("  Success: {data}"),
        Err(e) => println!("  {e}"),
    }

    println!("\nReport-nested:");
    match report_nested::handle_error(123) {
        Ok(data) => println!("  Success: {data}"),
        Err(e) => println!("  {e}"),
    }

    println!("\nFlat:");
    match flat::handle_error(123) {
        Ok(data) => println!("  Success: {data}"),
        Err(e) => println!("  {e}"),
    }

    println!("\nDynamic (Timeout):");
    match dynamic_propagation::handle_error(123) {
        Ok(data) => println!("  Success: {data}"),
        Err(e) => println!("  {e}"),
    }

    println!("\nDynamic (NotFound):");
    match dynamic_propagation::handle_error(404) {
        Ok(data) => println!("  Success: {data}"),
        Err(e) => println!("  {e}"),
    }
}
