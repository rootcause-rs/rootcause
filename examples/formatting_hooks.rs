//! Formatting hooks for global formatting overrides
//!
//! Formatting hooks vs handlers: Both customize how types are displayed, but:
//! - Handlers (see custom_handler.rs): Applied per-attachment with
//!   .attach_custom() or per-context
//! - Hooks (this example): Registered once globally and apply to all instances
//!   of a type
//!
//! Use formatting hooks to customize how types are displayed across your entire
//! application:
//! - AttachmentFormatterHook: Control placement (Inline/Appendix/Hidden) and
//!   priority
//! - ContextFormatterHook: Customize how error contexts are formatted

use rootcause::{
    ReportRef,
    handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
    hooks::{
        Hooks, attachment_formatter::AttachmentFormatterHook,
        context_formatter::ContextFormatterHook,
    },
    markers::{Dynamic, Local, Uncloneable},
    prelude::*,
    report_attachment::ReportAttachmentRef,
};

// Example 1: Attachment placement - control where diagnostic data appears

// Large diagnostic data that would clutter the main error message
struct DatabaseQuery {
    sql: String,
    params: Vec<String>,
    execution_plan: String,
}

impl core::fmt::Display for DatabaseQuery {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "SQL: {}", self.sql)?;
        writeln!(f, "Parameters: [{}]", self.params.join(", "))?;
        writeln!(f, "\nExecution Plan:")?;
        write!(f, "{}", self.execution_plan)
    }
}

impl core::fmt::Debug for DatabaseQuery {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DatabaseQuery {{ sql: {:?}, ... }}", self.sql)
    }
}

// Move verbose query diagnostics to appendix instead of cluttering inline
struct DatabaseQueryFormatter;

impl AttachmentFormatterHook<DatabaseQuery> for DatabaseQueryFormatter {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, Dynamic>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Appendix {
                appendix_name: "Database Query",
            },
            function: FormattingFunction::Display,
            priority: 0,
        }
    }
}

// Example 2: Attachment priority - control ordering

struct ActionRequired(String);

impl core::fmt::Display for ActionRequired {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "⚠️  ACTION REQUIRED: {}", self.0)
    }
}

impl core::fmt::Debug for ActionRequired {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// High priority ensures important actions appear first
struct ActionRequiredFormatter;

impl AttachmentFormatterHook<ActionRequired> for ActionRequiredFormatter {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, Dynamic>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Inline,
            function: FormattingFunction::Display,
            priority: 100, // High priority - shows before other attachments
        }
    }
}

// Example 3: Context formatting - customize error descriptions globally

#[derive(Debug)]
struct ValidationError {
    fields: Vec<(&'static str, &'static str)>,
}

impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} validation errors", self.fields.len())
    }
}

impl std::error::Error for ValidationError {}

// Custom formatting for validation errors across the app
struct ValidationErrorFormatter;

impl ContextFormatterHook<ValidationError> for ValidationErrorFormatter {
    fn display(
        &self,
        report: ReportRef<'_, ValidationError, Uncloneable, Local>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        let err = report.current_context();
        writeln!(f, "Validation failed:")?;
        for (field, reason) in &err.fields {
            writeln!(f, "  • {}: {}", field, reason)?;
        }
        Ok(())
    }
}

// Example 1: Control attachment placement in output
// Demonstrates placing verbose diagnostic data in the appendix section instead
// of inline
fn demo_attachment_placement() -> Result<(), Report> {
    let query = DatabaseQuery {
        sql: "SELECT * FROM users WHERE status = ? AND created_at > ?".to_string(),
        params: vec!["active".to_string(), "2024-01-01".to_string()],
        execution_plan: "Sequential Scan on users\n  Filter: (status = 'active')\n  Rows: 1234"
            .to_string(),
    };

    Err(report!("Database query failed")
        .attach(query)
        .attach("Connection timeout after 30s"))
}

fn demo_attachment_priority() -> Result<(), Report> {
    Err(report!("Configuration error")
        .attach("Debug info: config file not found")
        .attach(ActionRequired(
            "Create config.toml in project root".to_string(),
        ))
        .attach("Additional context here"))
}

fn demo_context_formatting() -> Result<(), Report> {
    let validation = ValidationError {
        fields: vec![("email", "invalid format"), ("age", "must be positive")],
    };

    Err(report!(validation).into_dynamic())
}

fn main() {
    // Install formatting hooks
    Hooks::new()
        .attachment_formatter::<DatabaseQuery, _>(DatabaseQueryFormatter)
        .attachment_formatter::<ActionRequired, _>(ActionRequiredFormatter)
        .context_formatter::<ValidationError, _>(ValidationErrorFormatter)
        .install()
        .expect("failed to install hooks");

    println!("Example 1: Attachment placement (Appendix)\n");
    match demo_attachment_placement() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Example 2: Attachment priority\n");
    match demo_attachment_priority() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Example 3: Context formatting\n");
    match demo_context_formatting() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}"),
    }
}
