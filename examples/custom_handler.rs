// Custom handlers for attachments and contexts
//
// Handlers vs formatting hooks (see formatting_hooks.rs):
// - Handlers (this example): Applied per-attachment/context with .attach_custom() or when creating the report
// - Formatting hooks: Registered once globally and apply to all instances of a type
//
// Use custom handlers to control formatting for:
// - Attachments: diagnostic data (logs, metrics, binary dumps)
// - Contexts: structured error descriptions (validation errors, API errors)

use rootcause::{
    handlers::{AttachmentHandler, ContextHandler},
    prelude::*,
};
use std::io;

// Example 1: Custom attachment handler for diagnostic data

struct BinaryData(Vec<u8>);

impl core::fmt::Display for BinaryData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} bytes", self.0.len())
    }
}

impl core::fmt::Debug for BinaryData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BinaryData({:?})", self.0)
    }
}

// Hexdump handler for binary diagnostic data
struct Hexdump;

impl AttachmentHandler<BinaryData> for Hexdump {
    fn display(data: &BinaryData, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Hexdump ({} bytes):", data.0.len())?;
        for (i, chunk) in data.0.chunks(16).enumerate() {
            write!(f, "{:04x}: ", i * 16)?;
            for byte in chunk {
                write!(f, "{:02x} ", byte)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    fn debug(data: &BinaryData, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(data, f)
    }
}

fn parse_protocol_message() -> Result<String, Report> {
    let corrupt_data = BinaryData(vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);

    Err(report!(io::Error::new(
        io::ErrorKind::InvalidData,
        "Protocol parse error"
    ))
    .attach("Received data:")
    .attach_custom::<Hexdump, _>(corrupt_data)
    .into_dyn_any())
}

// Example 2: Custom context handler for structured errors

struct ValidationError {
    fields: Vec<(&'static str, &'static str)>,
}

impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} validation error(s)", self.fields.len())
    }
}

impl core::fmt::Debug for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ValidationError")
            .field("fields", &self.fields)
            .finish()
    }
}

impl std::error::Error for ValidationError {}

// Pretty list handler for validation errors
struct ValidationList;

impl ContextHandler<ValidationError> for ValidationList {
    fn display(error: &ValidationError, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Validation failed:")?;
        for (field, reason) in &error.fields {
            writeln!(f, "  • {}: {}", field, reason)?;
        }
        Ok(())
    }

    fn debug(error: &ValidationError, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Self::display(error, f)
    }

    fn source(error: &ValidationError) -> Option<&(dyn std::error::Error + 'static)> {
        let _ = error;
        None
    }
}

fn validate_user_input() -> Result<(), Report> {
    let validation_error = ValidationError {
        fields: vec![
            ("email", "invalid format"),
            ("age", "must be positive"),
            ("username", "too short (min 3 chars)"),
        ],
    };

    // context_custom uses the custom handler for the error description
    Err(
        report!(io::Error::new(io::ErrorKind::InvalidInput, "Bad request"))
            .context_custom::<ValidationList, _>(validation_error)
            .into_dyn_any(),
    )
}

fn main() {
    println!("Example 1: Custom attachment handler\n");
    match parse_protocol_message() {
        Ok(_) => println!("Success"),
        Err(error) => eprintln!("{error}\n"),
    }

    println!("Example 2: Custom context handler\n");
    match validate_user_input() {
        Ok(()) => println!("Success"),
        Err(error) => eprintln!("{error}"),
    }
}
