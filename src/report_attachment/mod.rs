//! Individual attachments for error reports.
//!
//! This module provides types for creating and working with individual attachments
//! that can be added to error reports. Attachments allow you to include additional
//! context, data, or information alongside the main error message.
//!
//! # Core Types
//!
//! - [`ReportAttachment`]: An owned attachment that can be added to a report
//! - [`ReportAttachmentRef`]: A reference to an attachment, typically obtained from a report
//!
//! # Creating Attachments
//!
//! Attachments can be created from using [`ReportAttachment::new`] method or the [`report_attachment!`] macro:
//!
//! ```
//! use rootcause::{prelude::*, report_attachment::ReportAttachment};
//!
//! // Create a simple string attachment
//! let attachment: ReportAttachment<&str> = ReportAttachment::new("Additional information");
//! let attachment: ReportAttachment<dyn Any> = report_attachment!("Additional information");
//!
//! // Create an attachment from a custom type using Debug formatting
//! #[derive(Debug, Clone)]
//! struct AttachmentData {
//!     file: String,
//!     line: u32,
//! }
//!
//! let attachment_data = AttachmentData {
//!     file: "main.rs".to_string(),
//!     line: 42,
//! };
//! let debug_attachment: ReportAttachment<AttachmentData> =
//!     ReportAttachment::new_custom::<handlers::Debug>(attachment_data.clone());
//! let debug_attachment: ReportAttachment<AttachmentData> = report_attachment!(attachment_data);
//! ```
//!
//! # Type Parameters
//!
//! Both types have the same generic parameters:
//!
//! - **Attachment type**: Can be a concrete type or `dyn Any` for type erasure
//! - **Thread safety**: [`SendSync`] (default) for thread-safe attachments, or [`Local`] for single-threaded use
//!
//! ```
//! use std::rc::Rc;
//!
//! use rootcause::{markers, prelude::*, report_attachment::ReportAttachment};
//!
//! // Send + Sync attachment (default)
//! let attachment: ReportAttachment<String, markers::SendSync> =
//!     ReportAttachment::new("Thread-safe data".to_string());
//!
//! // Local-only attachment (cannot be sent between threads)
//! let local_data = Rc::new("Local data".to_string());
//! let local_attachment: ReportAttachment<Rc<String>, markers::Local> =
//!     ReportAttachment::new(local_data);
//!
//! // Send + Sync attachments can be converted to local-only
//! let local_conversion: ReportAttachment<String, markers::Local> = attachment.into_local();
//! ```
//!
//! # Attachment References
//!
//! When attachments are stored in reports, you typically work with [`ReportAttachmentRef`]
//! which provides access to the attachment data without taking ownership:
//!
//! ```
//! use rootcause::{prelude::*, report_attachment::ReportAttachment};
//!
//! let attachment = ReportAttachment::new("Important context");
//! let mut report = report!("An error occurred");
//! report.attachments_mut().push(attachment.into_dyn_any());
//!
//! // Access the attachment through a reference
//! let attachment_ref = report.attachments().get(0).unwrap();
//! println!("Attachment: {}", attachment_ref);
//! ```
//!
//! # Type Erasure and Downcasting
//!
//! Attachments support type erasure through `dyn Any`, allowing collections of
//! different attachment types. You can downcast back to concrete types when needed:
//!
//! ```
//! use std::any::{Any, TypeId};
//!
//! use rootcause::{prelude::*, report_attachment::ReportAttachment};
//!
//! let attachment: ReportAttachment<&str> = ReportAttachment::new("text data");
//! let erased: ReportAttachment<dyn Any> = attachment.into_dyn_any();
//!
//! // Check the type at runtime
//! assert_eq!(erased.inner_type_id(), TypeId::of::<&str>());
//!
//! // Safely downcast back to the original type
//! let attachment_ref = erased.as_ref();
//! let typed_ref = attachment_ref.downcast_attachment::<&str>().unwrap();
//! ```
//!
//! # Formatting and Handlers
//!
//! Attachments use handlers to control how they are formatted in reports.
//! The default handler uses the [`Display`] handler, but you can also use
//! the [`Debug`] handler or create your own:
//!
//! ```
//! # use core::any::Any;
//! use rootcause::{prelude::*, report_attachment::ReportAttachment};
//!
//! #[derive(Debug)]
//! struct MyData {
//!     value: i32,
//! }
//!
//! // Use Display formatting (default)
//! let display_attachment: ReportAttachment<&str> = ReportAttachment::new("text");
//!
//! // Use Debug formatting explicitly
//! let debug_attachment: ReportAttachment<MyData> =
//!     ReportAttachment::new_custom::<handlers::Debug>(MyData { value: 42 });
//!
//! struct MyOtherData {
//!     value: i32,
//! }
//!
//! struct MyDataHandler;
//! impl rootcause::handlers::AttachmentHandler<MyOtherData> for MyDataHandler {
//!     fn debug(data: &MyOtherData, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//!         write!(f, "MyOtherData value is: {}", data.value)
//!     }
//!
//!     fn display(data: &MyOtherData, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//!         write!(f, "MyOtherData value is: {}", data.value)
//!     }
//! }
//!
//! let custom_attachment: ReportAttachment<MyOtherData> =
//!     ReportAttachment::new_custom::<MyDataHandler>(MyOtherData { value: 100 });
//! ```
//!
//! [`SendSync`]: crate::markers::SendSync
//! [`Local`]: crate::markers::Local
//! [`Display`]: crate::handlers::Display
//! [`Debug`]: crate::handlers::Debug

mod owned;
mod ref_;

pub use self::{owned::ReportAttachment, ref_::ReportAttachmentRef};
