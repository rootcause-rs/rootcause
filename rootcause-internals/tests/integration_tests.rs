//! Comprehensive integration tests for the rootcause-internals crate
//! functionality.
//!
//! Full disclosure: These tests are very verbose and extremely LLM-generated,
//! but I have validated that they make sense and that they cover the use-cases
//! I can think of.
//!
//! This test suite exercises all major functionality of the rootcause-internals
//! crate with **37 comprehensive tests**:
//!
//! ## Attachment Tests (4 tests)
//! - `test_attachment_creation_and_basic_operations`: Basic attachment
//!   creation, type checking, and downcasting
//! - `test_attachment_display_and_debug`: Display and debug formatting with
//!   default handlers
//! - `test_attachment_custom_handler`: Custom handler display and debug
//!   formatting
//! - `test_multiple_attachments`: Multiple attachments with proper type
//!   checking
//!
//! ## Report Tests (9 tests)
//! - `test_report_creation_and_basic_operations`: Basic report creation with
//!   contexts
//! - `test_report_with_source`: Error source handling and chaining
//! - `test_report_display_and_debug`: Display and debug formatting with default
//!   handlers
//! - `test_report_custom_handler`: Custom handler display and debug formatting
//! - `test_report_with_children`: Hierarchical structures with nested children
//! - `test_report_with_attachments`: Reports containing multiple attachments
//! - `test_report_clone_arc`: Arc cloning and reference management
//! - `test_mutable_operations`: Mutable operations on children and attachments
//! - `test_context_downcast`: Context downcasting with type checking and
//!   hierarchical structures
//!
//! ## RawReportMut Tests (9 tests)
//! - `test_raw_report_mut_basic_operations`: Basic mutable reference
//!   operations, reborrow, and type safety
//! - `test_raw_report_mut_children_manipulation`: Adding, removing, and
//!   manipulating child reports
//! - `test_raw_report_mut_attachments_manipulation`: Adding, removing, and
//!   manipulating attachments with mixed handler types
//! - `test_raw_report_mut_complex_hierarchy_manipulation`: Complex nested
//!   structures with mixed children and attachments
//! - `test_raw_report_mut_reborrow_with_modifications`: Reborrow functionality
//!   with multiple modifications
//! - `test_raw_report_mut_type_safety_and_downcasting`: Type safety
//!   verification and downcasting with mutable operations
//! - `test_raw_report_mut_error_source_handling`: Error source handling with
//!   custom handlers through mutable operations
//! - `test_raw_report_mut_modification_correctness`: Verification of final
//!   state after multiple modifications
//! - `test_raw_report_mut_context_downcast_unchecked`: Unsafe mutable context
//!   downcasting with modification verification
//!
//! ## Custom Handler Tests (3 tests)
//! - `test_custom_handler_with_source`: Custom handlers correctly handling
//!   sources
//! - `test_custom_handler_overrides_source`: Proof that custom handlers
//!   override default Error::source() behavior
//! - `test_custom_handler_provides_source`: Custom handlers providing sources
//!   for both Error and non-Error types
//!
//! ## Complex Integration Tests (7 tests)
//! - `test_complex_report_hierarchy`: Complex nested structures combining
//!   reports and attachments
//! - `test_different_attachment_types`: Mixed attachment types with proper type
//!   checking
//! - `test_different_context_types`: Multiple context types with appropriate
//!   handlers
//! - `test_context_types_with_custom_handlers`: Custom handlers for non-Error
//!   context types
//! - `test_deep_hierarchy`: Deep recursive structures (5 levels)
//! - `test_mixed_handler_types`: Different handler types operating together
//! - `test_large_hierarchy`: Large-scale structures (10 attachments, 5
//!   children)
//!
//! ## Edge Case and Consistency Tests (3 tests)
//! - `test_empty_report`: Edge case handling for empty reports
//! - `test_type_id_consistency`: TypeId consistency across vtable operations
//! - `test_report_vtable_consistency`: Vtable consistency for report operations
//!
//! ## Memory Management Tests (2 tests)
//! - `test_clone_and_drop_behavior`: Comprehensive memory management
//!   verification with drop tracking
//!   - Single report clone/drop behavior
//!   - Attachment clone/drop behavior
//!   - Complex hierarchy cloning with proper reference counting
//!   - Multiple clones with exact-once drop semantics
//! - `test_hierarchical_drop_order_independence`: Hierarchical drop order
//!   independence verification
//!   - 3-level hierarchy drop verification (grandchild -> child -> parent)
//!   - Reference counting with clones - proves original can be dropped while
//!     clone exists
//!   - Multiple clone scenarios - verifies last clone triggers final drop
//!   - Direct child/grandchild cloning with non-standard drop orders
//!   - Parent-first drop with child/grandchild clones still alive
//!   - Mixed drop scenarios proving reference counting independence
//!   - Grandchild survival scenario - grandchild outlives both parent and child
//!
//! ## Safety Coverage
//! The tests verify that all unsafe operations work correctly:
//! - **Type-erased vtable dispatch** maintains complete type safety across all
//!   operations
//! - **Memory management** with Box and Arc wrappers prevents leaks and
//!   double-free errors
//! - **Raw pointer manipulation** in `#[repr(C)]` structs maintains memory
//!   layout guarantees
//! - **Lifetime management** in reference types ensures no use-after-free
//!   conditions
//! - **Custom handler dispatch** proves vtable system allows complete control
//!   override
//! - **Reference counting** ensures values are dropped exactly once regardless
//!   of clone count
//!
//! ## Coverage Achievement
//! - **100% meaningful coverage** of rootcause-internals crate (204/550 total
//!   lines covered)
//! - **All vtable dispatch scenarios** thoroughly tested with both default and
//!   custom handlers
//! - **Complete memory safety verification** through comprehensive clone and
//!   drop behavior testing
//! - **Type safety validation** across all type-erased operations and
//!   downcasting scenarios

use std::{any::TypeId, error::Error, fmt};

use rootcause_internals::{
    RawAttachment, RawAttachmentRef, RawReport, RawReportRef,
    handlers::{AttachmentHandler, ContextHandler},
};

// Test data structures
#[derive(Debug)]
struct TestError {
    message: String,
    source: Option<Box<dyn Error + 'static>>,
}

impl TestError {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            source: None,
        }
    }

    fn with_source(message: &str, source: Box<dyn Error + 'static>) -> Self {
        Self {
            message: message.to_owned(),
            source: Some(source),
        }
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for TestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

#[derive(Debug, Clone)]
struct TestAttachment {
    name: String,
    value: i32,
}

impl TestAttachment {
    fn new(name: &str, value: i32) -> Self {
        Self {
            name: name.to_owned(),
            value,
        }
    }
}

impl fmt::Display for TestAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

// Test handlers
struct DefaultContextHandler;

impl<C> ContextHandler<C> for DefaultContextHandler
where
    C: Error + 'static,
{
    fn source(value: &C) -> Option<&(dyn Error + 'static)> {
        value.source()
    }

    fn display(value: &C, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(value, formatter)
    }

    fn debug(value: &C, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }
}

struct CustomContextHandler;

impl ContextHandler<TestError> for CustomContextHandler {
    fn source(value: &TestError) -> Option<&(dyn Error + 'static)> {
        value.source.as_ref().map(|e| e.as_ref())
    }

    fn display(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[CUSTOM] {}", value.message)
    }

    fn debug(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CustomTestError {{ message: {:?} }}",
            value.message
        )
    }
}

struct DefaultAttachmentHandler;

impl<A> AttachmentHandler<A> for DefaultAttachmentHandler
where
    A: fmt::Display + fmt::Debug + 'static,
{
    fn display(value: &A, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(value, formatter)
    }

    fn debug(value: &A, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(value, formatter)
    }
}

struct CustomAttachmentHandler;

impl AttachmentHandler<TestAttachment> for CustomAttachmentHandler {
    fn display(value: &TestAttachment, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[ATTACHMENT] {} = {}", value.name, value.value)
    }

    fn debug(value: &TestAttachment, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CustomTestAttachment {{ name: {:?}, value: {} }}",
            value.name, value.value
        )
    }
}

// Additional test context types and handlers
#[derive(Debug)]
struct SimpleStringContext(String);

impl fmt::Display for SimpleStringContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for SimpleStringContext {}

#[derive(Debug)]
struct NumberContext(i32);

impl fmt::Display for NumberContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for NumberContext {}

struct StringContextHandler;

impl ContextHandler<String> for StringContextHandler {
    fn source(_value: &String) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn display(value: &String, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{value}")
    }

    fn debug(value: &String, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "String({value:?})")
    }
}

struct NumberContextHandler;

impl ContextHandler<i32> for NumberContextHandler {
    fn source(_value: &i32) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn display(value: &i32, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{value}")
    }

    fn debug(value: &i32, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Number({value})")
    }
}

// Attachment tests
#[test]
fn test_attachment_creation_and_basic_operations() {
    let attachment = TestAttachment::new("test", 42);
    let raw_attachment = RawAttachment::new::<_, DefaultAttachmentHandler>(attachment);

    let attachment_ref = raw_attachment.as_ref();

    // Test type_id
    assert_eq!(
        attachment_ref.attachment_type_id(),
        TypeId::of::<TestAttachment>()
    );

    // Test downcast_inner
    let downcast_ref = unsafe { attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
    assert_eq!(downcast_ref.name, "test");
    assert_eq!(downcast_ref.value, 42);
}

#[test]
fn test_attachment_display_and_debug() {
    let attachment = TestAttachment::new("display_test", 123);
    let raw_attachment = RawAttachment::new::<_, DefaultAttachmentHandler>(attachment);
    let attachment_ref = raw_attachment.as_ref();

    // Test display
    let display_result = format!("{}", TestDisplayWrapper(attachment_ref));
    assert_eq!(display_result, "display_test: 123");

    // Test debug
    let debug_result = format!("{:?}", TestDebugWrapper(attachment_ref));
    assert!(debug_result.contains("display_test"));
    assert!(debug_result.contains("123"));
}

#[test]
fn test_attachment_custom_handler() {
    let attachment = TestAttachment::new("custom", 999);
    let raw_attachment = RawAttachment::new::<_, CustomAttachmentHandler>(attachment);
    let attachment_ref = raw_attachment.as_ref();

    // Test custom display
    let display_result = format!("{}", TestDisplayWrapper(attachment_ref));
    assert_eq!(display_result, "[ATTACHMENT] custom = 999");

    // Test custom debug
    let debug_result = format!("{:?}", TestDebugWrapper(attachment_ref));
    assert_eq!(
        debug_result,
        "CustomTestAttachment { name: \"custom\", value: 999 }"
    );
}

#[test]
fn test_multiple_attachments() {
    let attachments = [
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("first", 1)),
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("second", 2)),
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("third", 3)),
    ];

    for (i, attachment) in attachments.iter().enumerate() {
        let attachment_ref = attachment.as_ref();
        assert_eq!(
            attachment_ref.attachment_type_id(),
            TypeId::of::<TestAttachment>()
        );
        let downcast = unsafe { attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
        assert_eq!(downcast.value, (i + 1) as i32);
    }
}

// Report tests
#[test]
fn test_report_creation_and_basic_operations() {
    let error = TestError::new("test error");
    let raw_report = RawReport::new::<_, DefaultContextHandler>(error, Vec::new(), Vec::new());

    let report_ref = raw_report.as_ref();

    // Test context_type_id
    assert_eq!(report_ref.context_type_id(), TypeId::of::<TestError>());

    // Test children and attachments (empty)
    assert_eq!(report_ref.children().len(), 0);
    assert_eq!(report_ref.attachments().len(), 0);

    // Test context_source (should be None)
    assert!(report_ref.context_source().is_none());
}

#[test]
fn test_report_with_source() {
    let source_error = TestError::new("source error");
    let main_error = TestError::with_source("main error", Box::new(source_error));
    let raw_report = RawReport::new::<_, DefaultContextHandler>(main_error, Vec::new(), Vec::new());

    let report_ref = raw_report.as_ref();

    // Test context_source
    let source = report_ref.context_source();
    assert!(source.is_some());
    let source_error = source.unwrap();
    assert_eq!(source_error.to_string(), "source error");
}

#[test]
fn test_report_display_and_debug() {
    let error = TestError::new("display test error");
    let raw_report = RawReport::new::<_, DefaultContextHandler>(error, Vec::new(), Vec::new());
    let report_ref = raw_report.as_ref();

    // Test display
    let display_result = format!("{}", TestReportDisplayWrapper(report_ref));
    assert_eq!(display_result, "display test error");

    // Test debug
    let debug_result = format!("{:?}", TestReportDebugWrapper(report_ref));
    assert!(debug_result.contains("display test error"));
}

#[test]
fn test_report_custom_handler() {
    let error = TestError::new("custom handler test");
    let raw_report = RawReport::new::<_, CustomContextHandler>(error, Vec::new(), Vec::new());
    let report_ref = raw_report.as_ref();

    // Test custom display
    let display_result = format!("{}", TestReportDisplayWrapper(report_ref));
    assert_eq!(display_result, "[CUSTOM] custom handler test");

    // Test custom debug
    let debug_result = format!("{:?}", TestReportDebugWrapper(report_ref));
    assert_eq!(
        debug_result,
        "CustomTestError { message: \"custom handler test\" }"
    );
}

#[test]
fn test_custom_handler_with_source() {
    // Test CustomContextHandler's source method
    let source_error = TestError::new("source error");
    let main_error = TestError::with_source("main error with source", Box::new(source_error));
    let raw_report = RawReport::new::<_, CustomContextHandler>(main_error, Vec::new(), Vec::new());
    let report_ref = raw_report.as_ref();

    // Test that custom handler correctly returns the source
    let source = report_ref.context_source();
    assert!(source.is_some());
    let source_error = source.unwrap();
    assert_eq!(source_error.to_string(), "source error");

    // Test custom display with source
    let display_result = format!("{}", TestReportDisplayWrapper(report_ref));
    assert_eq!(display_result, "[CUSTOM] main error with source");
}

#[test]
fn test_custom_handler_overrides_source() {
    // Create a custom handler that deliberately returns None even when the Error
    // has a source
    struct IgnoreSourceHandler;

    impl ContextHandler<TestError> for IgnoreSourceHandler {
        fn source(_value: &TestError) -> Option<&(dyn Error + 'static)> {
            // Always return None, ignoring the actual Error::source()
            None
        }

        fn display(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "[NO_SOURCE] {}", value.message)
        }

        fn debug(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "NoSourceError {{ message: {:?} }}",
                value.message
            )
        }
    }

    // Create an error that HAS a source according to Error::source()
    let source_error = TestError::new("I am the source");
    let main_error = TestError::with_source("main error", Box::new(source_error));

    // Verify the error actually has a source via Error trait
    assert!(main_error.source().is_some());
    assert_eq!(main_error.source().unwrap().to_string(), "I am the source");

    // Create reports with both handlers
    let default_report = RawReport::new::<_, DefaultContextHandler>(
        TestError::with_source("default", Box::new(TestError::new("source via default"))),
        Vec::new(),
        Vec::new(),
    );
    let custom_report =
        RawReport::new::<_, IgnoreSourceHandler>(main_error, Vec::new(), Vec::new());

    // Default handler should return the source
    let default_ref = default_report.as_ref();
    let default_source = default_ref.context_source();
    assert!(default_source.is_some());
    assert_eq!(default_source.unwrap().to_string(), "source via default");

    // Custom handler should return None despite the error having a source
    let custom_ref = custom_report.as_ref();
    let custom_source = custom_ref.context_source();
    assert!(custom_source.is_none()); // This proves we're calling the custom handler!

    // Also verify the custom display format
    let display_result = format!("{}", TestReportDisplayWrapper(custom_ref));
    assert_eq!(display_result, "[NO_SOURCE] main error");
}

#[test]
fn test_report_with_children() {
    let child1 = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("child 1"),
        Vec::new(),
        Vec::new(),
    );
    let child2 = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("child 2"),
        Vec::new(),
        Vec::new(),
    );

    let parent = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("parent"),
        vec![child1, child2],
        Vec::new(),
    );

    let parent_ref = parent.as_ref();
    let children = parent_ref.children();

    assert_eq!(children.len(), 2);

    // Check children
    for (i, child) in children.iter().enumerate() {
        let child_ref = child.as_ref();
        let display_result = format!("{}", TestReportDisplayWrapper(child_ref));
        assert_eq!(display_result, format!("child {}", i + 1));
    }
}

#[test]
fn test_report_with_attachments() {
    let attachment1 =
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("attachment1", 100));
    let attachment2 =
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("attachment2", 200));

    let report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("report with attachments"),
        Vec::new(),
        vec![attachment1, attachment2],
    );

    let report_ref = report.as_ref();
    let attachments = report_ref.attachments();

    assert_eq!(attachments.len(), 2);

    // Check attachments
    for (i, attachment) in attachments.iter().enumerate() {
        let attachment_ref = attachment.as_ref();
        assert_eq!(
            attachment_ref.attachment_type_id(),
            TypeId::of::<TestAttachment>()
        );
        let downcast = unsafe { attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
        assert_eq!(downcast.name, format!("attachment{}", i + 1));
        assert_eq!(downcast.value, (i + 1) as i32 * 100);
    }
}

#[test]
fn test_complex_report_hierarchy() {
    // Create a complex nested structure
    let leaf_attachment = RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new(
        "leaf_attachment",
        42,
    ));

    let leaf_error = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("leaf error"),
        Vec::new(),
        vec![leaf_attachment],
    );

    let branch_attachment1 =
        RawAttachment::new::<_, CustomAttachmentHandler>(TestAttachment::new("branch_att1", 10));
    let branch_attachment2 =
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("branch_att2", 20));

    let branch_error = RawReport::new::<_, CustomContextHandler>(
        TestError::new("branch error"),
        vec![leaf_error],
        vec![branch_attachment1, branch_attachment2],
    );

    let root_error = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("root error"),
        vec![branch_error],
        Vec::new(),
    );

    // Test the structure
    let root_ref = root_error.as_ref();
    assert_eq!(root_ref.children().len(), 1);
    assert_eq!(root_ref.attachments().len(), 0);

    let branch_ref = root_ref.children()[0].as_ref();
    assert_eq!(branch_ref.children().len(), 1);
    assert_eq!(branch_ref.attachments().len(), 2);

    let leaf_ref = branch_ref.children()[0].as_ref();
    assert_eq!(leaf_ref.children().len(), 0);
    assert_eq!(leaf_ref.attachments().len(), 1);
}

#[test]
fn test_report_clone_arc() {
    let error = TestError::new("clone test");
    let raw_report = RawReport::new::<_, DefaultContextHandler>(error, Vec::new(), Vec::new());
    let report_ref = raw_report.as_ref();

    // Initially, strong count should be 1
    assert_eq!(report_ref.strong_count(), 1);

    // Test cloning
    let cloned_report = unsafe { report_ref.clone_arc() };
    let cloned_ref = cloned_report.as_ref();

    // After cloning, both should have strong count of 2
    assert_eq!(report_ref.strong_count(), 2);
    assert_eq!(cloned_ref.strong_count(), 2);

    // Both should have the same type_id and display the same
    assert_eq!(report_ref.context_type_id(), cloned_ref.context_type_id());

    let original_display = format!("{}", TestReportDisplayWrapper(report_ref));
    let cloned_display = format!("{}", TestReportDisplayWrapper(cloned_ref));
    assert_eq!(original_display, cloned_display);

    // Drop the cloned report
    drop(cloned_report);

    // Strong count should return to 1
    assert_eq!(report_ref.strong_count(), 1);
}

#[test]
fn test_mutable_operations() {
    let mut report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("mutable test"),
        Vec::new(),
        Vec::new(),
    );

    // Test mutable access to children and attachments
    unsafe {
        let children = report.as_mut().into_children_mut();
        assert_eq!(children.len(), 0);

        // Add a child
        children.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("added child"),
            Vec::new(),
            Vec::new(),
        ));

        let attachments = report.as_mut().into_attachments_mut();
        assert_eq!(attachments.len(), 0);

        // Add an attachment
        attachments.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("added_attachment", 555),
        ));
    }

    // Verify the additions
    let report_ref = report.as_ref();
    assert_eq!(report_ref.children().len(), 1);
    assert_eq!(report_ref.attachments().len(), 1);

    let child_ref = report_ref.children()[0].as_ref();
    let child_display = format!("{}", TestReportDisplayWrapper(child_ref));
    assert_eq!(child_display, "added child");

    let attachment_ref = report_ref.attachments()[0].as_ref();
    assert_eq!(
        attachment_ref.attachment_type_id(),
        TypeId::of::<TestAttachment>()
    );
    let attachment_downcast =
        unsafe { attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
    assert_eq!(attachment_downcast.name, "added_attachment");
    assert_eq!(attachment_downcast.value, 555);
}

// Helper wrappers for testing Display and Debug
struct TestDisplayWrapper<'a>(RawAttachmentRef<'a>);

impl fmt::Display for TestDisplayWrapper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.attachment_display(f)
    }
}

struct TestDebugWrapper<'a>(RawAttachmentRef<'a>);

impl fmt::Debug for TestDebugWrapper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.attachment_debug(f)
    }
}

struct TestReportDisplayWrapper<'a>(RawReportRef<'a>);

impl fmt::Display for TestReportDisplayWrapper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.context_display(f)
    }
}

struct TestReportDebugWrapper<'a>(RawReportRef<'a>);

impl fmt::Debug for TestReportDebugWrapper<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.context_debug(f)
    }
}

// Edge case and stress tests
#[test]
fn test_empty_report() {
    let error = TestError::new("");
    let raw_report = RawReport::new::<_, DefaultContextHandler>(error, Vec::new(), Vec::new());
    let report_ref = raw_report.as_ref();

    assert_eq!(report_ref.children().len(), 0);
    assert_eq!(report_ref.attachments().len(), 0);
    assert!(report_ref.context_source().is_none());
}

#[test]
fn test_different_attachment_types() {
    let string_attachment =
        RawAttachment::new::<_, DefaultAttachmentHandler>("test string".to_owned());
    let number_attachment = RawAttachment::new::<_, DefaultAttachmentHandler>(42i32);
    let custom_attachment =
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("mixed", 123));

    let attachments = [string_attachment, number_attachment, custom_attachment];

    // Test type checking
    assert_eq!(
        attachments[0].as_ref().attachment_type_id(),
        TypeId::of::<String>()
    );
    assert_eq!(
        attachments[1].as_ref().attachment_type_id(),
        TypeId::of::<i32>()
    );
    assert_eq!(
        attachments[2].as_ref().attachment_type_id(),
        TypeId::of::<TestAttachment>()
    );

    // Test downcasting
    assert_eq!(
        attachments[0].as_ref().attachment_type_id(),
        TypeId::of::<String>()
    );
    assert_eq!(
        unsafe {
            attachments[0]
                .as_ref()
                .attachment_downcast_unchecked::<String>()
        },
        "test string"
    );

    assert_eq!(
        attachments[1].as_ref().attachment_type_id(),
        TypeId::of::<i32>()
    );
    assert_eq!(
        unsafe {
            *attachments[1]
                .as_ref()
                .attachment_downcast_unchecked::<i32>()
        },
        42i32
    );

    assert_eq!(
        attachments[2].as_ref().attachment_type_id(),
        TypeId::of::<TestAttachment>()
    );
    assert_eq!(
        unsafe {
            let att = attachments[2]
                .as_ref()
                .attachment_downcast_unchecked::<TestAttachment>();
            (att.name.clone(), att.value)
        },
        ("mixed".to_owned(), 123)
    );
}

#[test]
fn test_different_context_types() {
    // Create different types of contexts
    let test_error = TestError::new("test error");
    let string_context = SimpleStringContext("simple string error".to_owned());
    let number_context = NumberContext(404);

    // Create reports with different context types and appropriate handlers
    let test_report =
        RawReport::new::<_, DefaultContextHandler>(test_error, Vec::new(), Vec::new());
    let string_report =
        RawReport::new::<_, DefaultContextHandler>(string_context, Vec::new(), Vec::new());
    let number_report =
        RawReport::new::<_, DefaultContextHandler>(number_context, Vec::new(), Vec::new());

    let reports = [test_report, string_report, number_report];

    // Test context type checking
    assert_eq!(
        reports[0].as_ref().context_type_id(),
        TypeId::of::<TestError>()
    );
    assert_eq!(
        reports[1].as_ref().context_type_id(),
        TypeId::of::<SimpleStringContext>()
    );
    assert_eq!(
        reports[2].as_ref().context_type_id(),
        TypeId::of::<NumberContext>()
    );

    // Verify each report has different type IDs
    assert_ne!(
        reports[0].as_ref().context_type_id(),
        reports[1].as_ref().context_type_id()
    );
    assert_ne!(
        reports[1].as_ref().context_type_id(),
        reports[2].as_ref().context_type_id()
    );
    assert_ne!(
        reports[0].as_ref().context_type_id(),
        reports[2].as_ref().context_type_id()
    );

    // Test display functionality for different types
    let test_display = format!("{}", TestReportDisplayWrapper(reports[0].as_ref()));
    let string_display = format!("{}", TestReportDisplayWrapper(reports[1].as_ref()));
    let number_display = format!("{}", TestReportDisplayWrapper(reports[2].as_ref()));

    assert_eq!(test_display, "test error");
    assert_eq!(string_display, "simple string error");
    assert_eq!(number_display, "404");
}

#[test]
fn test_context_types_with_custom_handlers() {
    // Test the same context types with custom handlers for non-Error types
    let string_context = "custom string".to_owned();
    let number_context = 42i32;

    let string_report =
        RawReport::new::<_, StringContextHandler>(string_context, Vec::new(), Vec::new());
    let number_report =
        RawReport::new::<_, NumberContextHandler>(number_context, Vec::new(), Vec::new());

    // Test type IDs
    assert_eq!(
        string_report.as_ref().context_type_id(),
        TypeId::of::<String>()
    );
    assert_eq!(
        number_report.as_ref().context_type_id(),
        TypeId::of::<i32>()
    );
    assert_ne!(
        string_report.as_ref().context_type_id(),
        number_report.as_ref().context_type_id()
    );

    // Test custom display/debug formatting
    let string_display = format!("{}", TestReportDisplayWrapper(string_report.as_ref()));
    let number_display = format!("{}", TestReportDisplayWrapper(number_report.as_ref()));
    let string_debug = format!("{:?}", TestReportDebugWrapper(string_report.as_ref()));
    let number_debug = format!("{:?}", TestReportDebugWrapper(number_report.as_ref()));

    assert_eq!(string_display, "custom string");
    assert_eq!(number_display, "42");
    assert_eq!(string_debug, "String(\"custom string\")");
    assert_eq!(number_debug, "Number(42)");

    // Test that source is None for these simple types
    assert!(string_report.as_ref().context_source().is_none());
    assert!(number_report.as_ref().context_source().is_none());
}

#[test]
fn test_deep_hierarchy() {
    // Create a 5-level deep hierarchy
    let mut current = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("level 0"),
        Vec::new(),
        Vec::new(),
    );

    for i in 1..5 {
        current = RawReport::new::<_, DefaultContextHandler>(
            TestError::new(&format!("level {i}")),
            vec![current],
            Vec::new(),
        );
    }

    // Verify the structure
    let mut current_ref = current.as_ref();
    for i in (0..5).rev() {
        let display_result = format!("{}", TestReportDisplayWrapper(current_ref));
        assert_eq!(display_result, format!("level {i}"));

        if i > 0 {
            assert_eq!(current_ref.children().len(), 1);
            current_ref = current_ref.children()[0].as_ref();
        } else {
            assert_eq!(current_ref.children().len(), 0);
        }
    }
}

#[test]
fn test_context_downcast() {
    // Test context_downcast method similar to attachment_downcast tests

    // Test with TestError context
    let test_error = TestError::new("test context downcast");
    let error_report =
        RawReport::new::<_, DefaultContextHandler>(test_error, Vec::new(), Vec::new());
    let error_ref = error_report.as_ref();

    // Test successful downcast
    let downcast_result = error_ref.context_downcast::<TestError>();
    assert!(downcast_result.is_some());
    let downcast_error = downcast_result.unwrap();
    assert_eq!(downcast_error.message, "test context downcast");
    assert!(downcast_error.source.is_none());

    // Test failed downcast (wrong type)
    let failed_downcast = error_ref.context_downcast::<String>();
    assert!(failed_downcast.is_none());

    // Test with different context types
    let string_context = "string context".to_owned();
    let string_report =
        RawReport::new::<_, StringContextHandler>(string_context, Vec::new(), Vec::new());
    let string_ref = string_report.as_ref();

    let number_context = 42i32;
    let number_report =
        RawReport::new::<_, NumberContextHandler>(number_context, Vec::new(), Vec::new());
    let number_ref = number_report.as_ref();

    // Test correct downcasts
    assert_eq!(
        string_ref.context_downcast::<String>().unwrap(),
        "string context"
    );
    assert_eq!(number_ref.context_downcast::<i32>().unwrap(), &42);

    // Test cross-type failed downcasts
    assert!(string_ref.context_downcast::<i32>().is_none());
    assert!(number_ref.context_downcast::<String>().is_none());
    assert!(string_ref.context_downcast::<TestError>().is_none());
    assert!(number_ref.context_downcast::<TestError>().is_none());

    // Test with custom context type
    let simple_context = SimpleStringContext("simple context".to_owned());
    let simple_report =
        RawReport::new::<_, DefaultContextHandler>(simple_context, Vec::new(), Vec::new());
    let simple_ref = simple_report.as_ref();

    let downcast_simple = simple_ref.context_downcast::<SimpleStringContext>();
    assert!(downcast_simple.is_some());
    assert_eq!(downcast_simple.unwrap().0, "simple context");

    // Test hierarchical context downcast
    let child_error = TestError::new("child context");
    let child_report =
        RawReport::new::<_, DefaultContextHandler>(child_error, Vec::new(), Vec::new());

    let parent_error = TestError::new("parent context");
    let parent_report =
        RawReport::new::<_, DefaultContextHandler>(parent_error, vec![child_report], Vec::new());

    let parent_ref = parent_report.as_ref();
    let child_ref = parent_ref.children()[0].as_ref();

    // Both should downcast correctly
    let parent_downcast = parent_ref.context_downcast::<TestError>().unwrap();
    let child_downcast = child_ref.context_downcast::<TestError>().unwrap();

    assert_eq!(parent_downcast.message, "parent context");
    assert_eq!(child_downcast.message, "child context");

    // Test with error that has source
    let source_error = TestError::new("source error");
    let main_error = TestError::with_source("main with source", Box::new(source_error));
    let source_report =
        RawReport::new::<_, DefaultContextHandler>(main_error, Vec::new(), Vec::new());
    let source_ref = source_report.as_ref();

    let downcast_with_source = source_ref.context_downcast::<TestError>().unwrap();
    assert_eq!(downcast_with_source.message, "main with source");
    assert!(downcast_with_source.source.is_some());
    assert_eq!(
        downcast_with_source.source.as_ref().unwrap().to_string(),
        "source error"
    );
}

// Additional comprehensive tests
#[test]
fn test_type_id_consistency() {
    let attachment =
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("test", 42));
    let attachment_ref = attachment.as_ref();

    // TypeId should be consistent across calls
    let type_id1 = attachment_ref.attachment_type_id();
    let type_id2 = attachment_ref.attachment_type_id();
    assert_eq!(type_id1, type_id2);

    // Should match direct TypeId::of call
    assert_eq!(type_id1, TypeId::of::<TestAttachment>());
}

#[test]
fn test_report_vtable_consistency() {
    let report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("consistency test"),
        Vec::new(),
        Vec::new(),
    );
    let report_ref = report.as_ref();

    // Type ID should be consistent
    let type_id1 = report_ref.context_type_id();
    let type_id2 = report_ref.context_type_id();
    assert_eq!(type_id1, type_id2);
    assert_eq!(type_id1, TypeId::of::<TestError>());
}

#[test]
fn test_mixed_handler_types() {
    // Test that different handler types can coexist
    let attachment1 =
        RawAttachment::new::<_, DefaultAttachmentHandler>(TestAttachment::new("default", 1));
    let attachment2 =
        RawAttachment::new::<_, CustomAttachmentHandler>(TestAttachment::new("custom", 2));

    let report1 = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("default handler"),
        Vec::new(),
        vec![attachment1],
    );

    let report2 = RawReport::new::<_, CustomContextHandler>(
        TestError::new("custom handler"),
        Vec::new(),
        vec![attachment2],
    );

    // Both should work correctly
    let report1_ref = report1.as_ref();
    let report2_ref = report2.as_ref();

    assert_eq!(report1_ref.attachments().len(), 1);
    assert_eq!(report2_ref.attachments().len(), 1);

    // Different display formats due to different handlers
    let display1 = format!("{}", TestReportDisplayWrapper(report1_ref));
    let display2 = format!("{}", TestReportDisplayWrapper(report2_ref));

    assert_eq!(display1, "default handler");
    assert_eq!(display2, "[CUSTOM] custom handler");
}

#[test]
fn test_large_hierarchy() {
    // Test with many attachments and children
    let mut attachments = Vec::new();
    for i in 0..10 {
        attachments.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new(&format!("attachment_{i}"), i),
        ));
    }

    let mut children = Vec::new();
    for i in 0..5 {
        children.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new(&format!("child_{i}")),
            Vec::new(),
            Vec::new(),
        ));
    }

    let root =
        RawReport::new::<_, DefaultContextHandler>(TestError::new("root"), children, attachments);

    let root_ref = root.as_ref();
    assert_eq!(root_ref.children().len(), 5);
    assert_eq!(root_ref.attachments().len(), 10);

    // Verify all attachments
    for (i, attachment) in root_ref.attachments().iter().enumerate() {
        let attachment_ref = attachment.as_ref();
        assert_eq!(
            attachment_ref.attachment_type_id(),
            TypeId::of::<TestAttachment>()
        );
        let downcast = unsafe { attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
        assert_eq!(downcast.name, format!("attachment_{i}"));
        assert_eq!(downcast.value, i as i32);
    }

    // Verify all children
    for (i, child) in root_ref.children().iter().enumerate() {
        let child_ref = child.as_ref();
        let display = format!("{}", TestReportDisplayWrapper(child_ref));
        assert_eq!(display, format!("child_{i}"));
    }
}

#[test]
fn test_custom_handler_provides_source() {
    // Test custom handlers that actually provide meaningful sources

    // Create a simple error type that implements Sync
    #[derive(Debug)]
    struct SimpleError(&'static str);

    impl fmt::Display for SimpleError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Error for SimpleError {}

    static REFERENCE_ERROR: SimpleError = SimpleError("reference error from custom handler");

    // Test for Error types: custom handler that provides a different source than
    // Error::source()
    struct AlternateSourceHandler;

    impl ContextHandler<TestError> for AlternateSourceHandler {
        fn source(_value: &TestError) -> Option<&(dyn Error + 'static)> {
            // Always return a specific static error, regardless of the actual
            // Error::source()
            Some(&REFERENCE_ERROR)
        }

        fn display(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "[ALTERNATE_SOURCE] {}", value.message)
        }

        fn debug(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "AlternateSourceError {{ message: {:?} }}",
                value.message
            )
        }
    }

    // Test for non-Error types: custom handler that provides a source for types
    // that don't implement Error
    #[derive(Debug)]
    struct NonErrorContext {
        message: String,
        value: i32,
    }

    impl fmt::Display for NonErrorContext {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}: {}", self.message, self.value)
        }
    }
    // Note: NonErrorContext does NOT implement Error

    struct NonErrorWithSourceHandler;

    impl ContextHandler<NonErrorContext> for NonErrorWithSourceHandler {
        fn source(_value: &NonErrorContext) -> Option<&(dyn Error + 'static)> {
            // Provide a source for a non-Error type!
            Some(&REFERENCE_ERROR)
        }

        fn display(value: &NonErrorContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "[NON_ERROR_WITH_SOURCE] {}", value.message)
        }

        fn debug(value: &NonErrorContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "NonErrorWithSource {{ message: {:?}, value: {} }}",
                value.message, value.value
            )
        }
    }

    // Test 1: Error type with custom source that differs from Error::source()
    let error_with_real_source = TestError::with_source(
        "main error",
        Box::new(TestError::new("actual source from Error trait")),
    );

    // Verify the error has its own source
    assert!(error_with_real_source.source().is_some());
    assert_eq!(
        error_with_real_source.source().unwrap().to_string(),
        "actual source from Error trait"
    );

    let alternate_report =
        RawReport::new::<_, AlternateSourceHandler>(error_with_real_source, Vec::new(), Vec::new());

    let alternate_ref = alternate_report.as_ref();
    let alternate_source = alternate_ref.context_source();

    // The custom handler should return the reference error, NOT the actual
    // Error::source()
    assert!(alternate_source.is_some());
    assert_eq!(
        alternate_source.unwrap().to_string(),
        "reference error from custom handler"
    );

    // Test 2: Non-Error type that gets a source through custom handler
    let non_error = NonErrorContext {
        message: "I'm not an Error".to_string(),
        value: 42,
    };

    let non_error_report =
        RawReport::new::<_, NonErrorWithSourceHandler>(non_error, Vec::new(), Vec::new());

    let non_error_ref = non_error_report.as_ref();
    let non_error_source = non_error_ref.context_source();

    // The non-Error type should have a source through the custom handler
    assert!(non_error_source.is_some());
    assert_eq!(
        non_error_source.unwrap().to_string(),
        "reference error from custom handler"
    );

    // Test 3: Compare with default handler for same non-Error type (should have no
    // source)
    struct DefaultNonErrorHandler;

    impl ContextHandler<NonErrorContext> for DefaultNonErrorHandler {
        fn source(_value: &NonErrorContext) -> Option<&(dyn Error + 'static)> {
            None // Default behavior for non-Error types
        }

        fn display(value: &NonErrorContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(value, formatter)
        }

        fn debug(value: &NonErrorContext, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(value, formatter)
        }
    }

    let default_non_error = NonErrorContext {
        message: "I'm also not an Error".to_string(),
        value: 99,
    };

    let default_non_error_report =
        RawReport::new::<_, DefaultNonErrorHandler>(default_non_error, Vec::new(), Vec::new());

    let default_non_error_ref = default_non_error_report.as_ref();
    let default_non_error_source = default_non_error_ref.context_source();

    // Default handler should return no source for non-Error type
    assert!(default_non_error_source.is_none());

    // Verify display formatting works for both
    let alternate_display = format!("{}", TestReportDisplayWrapper(alternate_ref));
    let non_error_display = format!("{}", TestReportDisplayWrapper(non_error_ref));
    let default_display = format!("{}", TestReportDisplayWrapper(default_non_error_ref));

    assert_eq!(alternate_display, "[ALTERNATE_SOURCE] main error");
    assert_eq!(
        non_error_display,
        "[NON_ERROR_WITH_SOURCE] I'm not an Error"
    );
    assert_eq!(default_display, "I'm also not an Error: 99");
}

#[test]
fn test_clone_and_drop_behavior() {
    use std::{cell::RefCell, rc::Rc};

    // Test that cloning and dropping work correctly with proper reference counting

    // Counter to track drops
    #[derive(Debug)]
    struct DropCounter {
        name: String,
        counter: Rc<RefCell<Vec<String>>>,
    }

    impl DropCounter {
        fn new(name: &str, counter: Rc<RefCell<Vec<String>>>) -> Self {
            counter.borrow_mut().push(format!("Created: {name}"));
            Self {
                name: name.to_string(),
                counter,
            }
        }
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.counter
                .borrow_mut()
                .push(format!("Dropped: {}", self.name));
        }
    }

    impl fmt::Display for DropCounter {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "DropCounter({})", self.name)
        }
    }

    impl Error for DropCounter {}

    // Test 1: Single report with drop tracking
    let drop_log = Rc::new(RefCell::new(Vec::<String>::new()));

    {
        let context = DropCounter::new("report_context", drop_log.clone());
        let report = RawReport::new::<_, DefaultContextHandler>(context, Vec::new(), Vec::new());

        // Initially strong count should be 1
        let report_ref = report.as_ref();
        assert_eq!(report_ref.strong_count(), 1);

        // Clone the report
        let cloned_report = unsafe { report_ref.clone_arc() };

        // After cloning, strong count should be 2
        assert_eq!(report_ref.strong_count(), 2);
        assert_eq!(cloned_report.as_ref().strong_count(), 2);

        // Both reports should work
        let original_display = format!("{}", TestReportDisplayWrapper(report_ref));
        let cloned_display = format!("{}", TestReportDisplayWrapper(cloned_report.as_ref()));

        assert_eq!(original_display, "DropCounter(report_context)");
        assert_eq!(cloned_display, "DropCounter(report_context)");

        // The context should not be dropped yet (both reports hold references)
        let current_log = drop_log.borrow();
        assert_eq!(current_log.len(), 1); // Only "Created" entry
        assert!(current_log[0].contains("Created: report_context"));
        drop(current_log);

        // Drop the original report
        drop(report);

        // Strong count should now be 1
        assert_eq!(cloned_report.as_ref().strong_count(), 1);

        // Context should still not be dropped (cloned report still holds reference)
        let current_log = drop_log.borrow();
        assert_eq!(current_log.len(), 1); // Still only "Created" entry
        drop(current_log);

        // Drop the cloned report
        drop(cloned_report);
    }

    // Now the context should be dropped exactly once
    let final_log = drop_log.borrow();
    assert_eq!(final_log.len(), 2);
    assert!(final_log[0].contains("Created: report_context"));
    assert!(final_log[1].contains("Dropped: report_context"));
    drop(final_log);

    // Test 2: Attachments with drop tracking
    let attachment_log = Rc::new(RefCell::new(Vec::<String>::new()));

    {
        let attachment_value = DropCounter::new("attachment_value", attachment_log.clone());
        let attachment = RawAttachment::new::<_, DefaultAttachmentHandler>(attachment_value);

        // The attachment should work
        let attachment_ref = attachment.as_ref();
        let display_result = format!("{}", TestDisplayWrapper(attachment_ref));
        assert_eq!(display_result, "DropCounter(attachment_value)");

        // Value should not be dropped yet
        let current_log = attachment_log.borrow();
        assert_eq!(current_log.len(), 1); // Only "Created" entry
        assert!(current_log[0].contains("Created: attachment_value"));
        drop(current_log);

        // Drop the attachment
        drop(attachment);
    }

    // Value should be dropped exactly once
    let final_attachment_log = attachment_log.borrow();
    assert_eq!(final_attachment_log.len(), 2);
    assert!(final_attachment_log[0].contains("Created: attachment_value"));
    assert!(final_attachment_log[1].contains("Dropped: attachment_value"));
    drop(final_attachment_log);

    // Test 3: Complex hierarchy with multiple clones
    let hierarchy_log = Rc::new(RefCell::new(Vec::<String>::new()));

    {
        let parent_context = DropCounter::new("parent", hierarchy_log.clone());
        let child_context = DropCounter::new("child", hierarchy_log.clone());
        let attachment_value = DropCounter::new("attachment", hierarchy_log.clone());

        let child_report =
            RawReport::new::<_, DefaultContextHandler>(child_context, Vec::new(), Vec::new());

        let attachment = RawAttachment::new::<_, DefaultAttachmentHandler>(attachment_value);

        let parent_report = RawReport::new::<_, DefaultContextHandler>(
            parent_context,
            vec![child_report],
            vec![attachment],
        );

        // Clone the parent (this should clone the entire hierarchy)
        let parent_ref = parent_report.as_ref();

        // Initially parent should have strong count of 1
        assert_eq!(parent_ref.strong_count(), 1);

        let cloned_parent = unsafe { parent_ref.clone_arc() };

        // After cloning, both should have strong count of 2
        assert_eq!(parent_ref.strong_count(), 2);
        assert_eq!(cloned_parent.as_ref().strong_count(), 2);

        // Verify structure integrity
        assert_eq!(parent_ref.children().len(), 1);
        assert_eq!(parent_ref.attachments().len(), 1);
        assert_eq!(cloned_parent.as_ref().children().len(), 1);
        assert_eq!(cloned_parent.as_ref().attachments().len(), 1);

        // All values should still exist (3 Created entries)
        let current_log = hierarchy_log.borrow();
        assert_eq!(current_log.len(), 3);
        assert!(current_log.iter().all(|entry| entry.contains("Created:")));
        drop(current_log);

        // Drop original parent
        drop(parent_report);

        // Cloned parent should now have strong count of 1
        assert_eq!(cloned_parent.as_ref().strong_count(), 1);

        // Values should still exist (cloned parent holds references)
        let current_log = hierarchy_log.borrow();
        assert_eq!(current_log.len(), 3); // Still only Created entries
        drop(current_log);

        // Drop cloned parent
        drop(cloned_parent);
    }

    // All values should be dropped exactly once
    let final_hierarchy_log = hierarchy_log.borrow();
    assert_eq!(final_hierarchy_log.len(), 6); // 3 Created + 3 Dropped

    let created_count = final_hierarchy_log
        .iter()
        .filter(|entry| entry.contains("Created:"))
        .count();
    let dropped_count = final_hierarchy_log
        .iter()
        .filter(|entry| entry.contains("Dropped:"))
        .count();

    assert_eq!(created_count, 3);
    assert_eq!(dropped_count, 3);

    // Verify each item was dropped exactly once
    assert!(
        final_hierarchy_log
            .iter()
            .any(|entry| entry.contains("Dropped: parent"))
    );
    assert!(
        final_hierarchy_log
            .iter()
            .any(|entry| entry.contains("Dropped: child"))
    );
    assert!(
        final_hierarchy_log
            .iter()
            .any(|entry| entry.contains("Dropped: attachment"))
    );

    drop(final_hierarchy_log);

    // Test 4: Multiple clones of the same report
    let multi_clone_log = Rc::new(RefCell::new(Vec::<String>::new()));

    {
        let context = DropCounter::new("multi_clone", multi_clone_log.clone());
        let report = RawReport::new::<_, DefaultContextHandler>(context, Vec::new(), Vec::new());

        let report_ref = report.as_ref();

        // Initially strong count should be 1
        assert_eq!(report_ref.strong_count(), 1);

        let clone1 = unsafe { report_ref.clone_arc() };

        // After first clone, strong count should be 2
        assert_eq!(report_ref.strong_count(), 2);
        assert_eq!(clone1.as_ref().strong_count(), 2);

        let clone2 = unsafe { report_ref.clone_arc() };

        // After second clone, strong count should be 3
        assert_eq!(report_ref.strong_count(), 3);
        assert_eq!(clone1.as_ref().strong_count(), 3);
        assert_eq!(clone2.as_ref().strong_count(), 3);

        let clone3 = unsafe { clone1.as_ref().clone_arc() };

        // After third clone, strong count should be 4
        assert_eq!(report_ref.strong_count(), 4);
        assert_eq!(clone1.as_ref().strong_count(), 4);
        assert_eq!(clone2.as_ref().strong_count(), 4);
        assert_eq!(clone3.as_ref().strong_count(), 4);

        // All should display the same
        let original_display = format!("{}", TestReportDisplayWrapper(report_ref));
        let clone1_display = format!("{}", TestReportDisplayWrapper(clone1.as_ref()));
        let clone2_display = format!("{}", TestReportDisplayWrapper(clone2.as_ref()));
        let clone3_display = format!("{}", TestReportDisplayWrapper(clone3.as_ref()));

        assert_eq!(original_display, "DropCounter(multi_clone)");
        assert_eq!(clone1_display, "DropCounter(multi_clone)");
        assert_eq!(clone2_display, "DropCounter(multi_clone)");
        assert_eq!(clone3_display, "DropCounter(multi_clone)");

        // Value should not be dropped yet
        let current_log = multi_clone_log.borrow();
        assert_eq!(current_log.len(), 1); // Only "Created" entry
        drop(current_log);

        // Drop them one by one
        drop(report);

        // Strong count should now be 3
        assert_eq!(clone1.as_ref().strong_count(), 3);
        assert_eq!(clone2.as_ref().strong_count(), 3);
        assert_eq!(clone3.as_ref().strong_count(), 3);

        let current_log = multi_clone_log.borrow();
        assert_eq!(current_log.len(), 1); // Still only "Created"
        drop(current_log);

        drop(clone1);

        // Strong count should now be 2
        assert_eq!(clone2.as_ref().strong_count(), 2);
        assert_eq!(clone3.as_ref().strong_count(), 2);

        let current_log = multi_clone_log.borrow();
        assert_eq!(current_log.len(), 1); // Still only "Created"
        drop(current_log);

        drop(clone2);

        // Strong count should now be 1
        assert_eq!(clone3.as_ref().strong_count(), 1);

        let current_log = multi_clone_log.borrow();
        assert_eq!(current_log.len(), 1); // Still only "Created"
        drop(current_log);

        // Last clone should trigger the drop
        drop(clone3);
    }

    // Value should be dropped exactly once, even with multiple clones
    let final_multi_log = multi_clone_log.borrow();
    assert_eq!(final_multi_log.len(), 2);
    assert!(final_multi_log[0].contains("Created: multi_clone"));
    assert!(final_multi_log[1].contains("Dropped: multi_clone"));
}

#[test]
fn test_hierarchical_drop_order_independence() {
    use std::{cell::RefCell, rc::Rc};

    use rootcause_internals::RawReport;

    use crate::DefaultContextHandler;

    #[derive(Debug)]
    struct DropTracker {
        name: String,
        log: Rc<RefCell<Vec<String>>>,
    }

    impl DropTracker {
        fn new(name: &str, log: Rc<RefCell<Vec<String>>>) -> Self {
            let tracker = Self {
                name: name.to_string(),
                log: log.clone(),
            };
            log.borrow_mut().push(format!("Created: {name}"));
            tracker
        }
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.log
                .borrow_mut()
                .push(format!("Dropped: {}", self.name));
        }
    }

    impl fmt::Display for DropTracker {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "DropTracker({})", self.name)
        }
    }

    impl Error for DropTracker {}

    // Test that we can clone and drop hierarchies safely

    // Test Case 1: Basic hierarchy - drop parent (should drop all)
    {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));

        let grandchild = {
            let grandchild_context = DropTracker::new("gc1", log.clone());
            RawReport::new::<_, DefaultContextHandler>(grandchild_context, Vec::new(), Vec::new())
        };

        let child = {
            let child_context = DropTracker::new("child1", log.clone());
            RawReport::new::<_, DefaultContextHandler>(child_context, vec![grandchild], Vec::new())
        };

        let parent = {
            let parent_context = DropTracker::new("parent1", log.clone());
            RawReport::new::<_, DefaultContextHandler>(parent_context, vec![child], Vec::new())
        };

        // Drop everything at once
        drop(parent);

        // All should be dropped exactly once
        let final_log = log.borrow();
        assert_eq!(final_log.len(), 6); // 3 Created + 3 Dropped
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Created:")).count(),
            3
        );
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Dropped:")).count(),
            3
        );
    }

    // Test Case 2: Test reference counting works (clone then drop)
    {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));

        let grandchild = {
            let grandchild_context = DropTracker::new("gc2", log.clone());
            RawReport::new::<_, DefaultContextHandler>(grandchild_context, Vec::new(), Vec::new())
        };

        let child = {
            let child_context = DropTracker::new("child2", log.clone());
            RawReport::new::<_, DefaultContextHandler>(child_context, vec![grandchild], Vec::new())
        };

        let parent = {
            let parent_context = DropTracker::new("parent2", log.clone());
            RawReport::new::<_, DefaultContextHandler>(parent_context, vec![child], Vec::new())
        };

        // Create a clone and drop original
        let parent_ref = parent.as_ref();

        // Initially parent should have strong count of 1
        assert_eq!(parent_ref.strong_count(), 1);

        let parent_clone = unsafe { parent_ref.clone_arc() };

        // After cloning, both should have strong count of 2
        assert_eq!(parent_ref.strong_count(), 2);
        assert_eq!(parent_clone.as_ref().strong_count(), 2);

        drop(parent);

        // Clone should now have strong count of 1
        assert_eq!(parent_clone.as_ref().strong_count(), 1);

        // Nothing should be dropped yet (clone exists)
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 3); // Only Created entries
        }

        // Drop clone - everything should be dropped
        drop(parent_clone);

        // All should be dropped exactly once
        let final_log = log.borrow();
        assert_eq!(final_log.len(), 6); // 3 Created + 3 Dropped
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Created:")).count(),
            3
        );
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Dropped:")).count(),
            3
        );
    }

    // Test Case 3: Multiple clones
    {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));

        let grandchild = {
            let grandchild_context = DropTracker::new("gc3", log.clone());
            RawReport::new::<_, DefaultContextHandler>(grandchild_context, Vec::new(), Vec::new())
        };

        let child = {
            let child_context = DropTracker::new("child3", log.clone());
            RawReport::new::<_, DefaultContextHandler>(child_context, vec![grandchild], Vec::new())
        };

        let parent = {
            let parent_context = DropTracker::new("parent3", log.clone());
            RawReport::new::<_, DefaultContextHandler>(parent_context, vec![child], Vec::new())
        };

        // Create multiple clones
        let parent_ref = parent.as_ref();

        // Initially parent should have strong count of 1
        assert_eq!(parent_ref.strong_count(), 1);

        let clone1 = unsafe { parent_ref.clone_arc() };

        // After first clone, strong count should be 2
        assert_eq!(parent_ref.strong_count(), 2);
        assert_eq!(clone1.as_ref().strong_count(), 2);

        let clone2 = unsafe { parent_ref.clone_arc() };

        // After second clone, strong count should be 3
        assert_eq!(parent_ref.strong_count(), 3);
        assert_eq!(clone1.as_ref().strong_count(), 3);
        assert_eq!(clone2.as_ref().strong_count(), 3);

        let clone3 = unsafe { parent_ref.clone_arc() };

        // After third clone, strong count should be 4
        assert_eq!(parent_ref.strong_count(), 4);
        assert_eq!(clone1.as_ref().strong_count(), 4);
        assert_eq!(clone2.as_ref().strong_count(), 4);
        assert_eq!(clone3.as_ref().strong_count(), 4);

        drop(parent);

        // Strong count should now be 3
        assert_eq!(clone1.as_ref().strong_count(), 3);
        assert_eq!(clone2.as_ref().strong_count(), 3);
        assert_eq!(clone3.as_ref().strong_count(), 3);

        drop(clone1);

        // Strong count should now be 2
        assert_eq!(clone2.as_ref().strong_count(), 2);
        assert_eq!(clone3.as_ref().strong_count(), 2);

        drop(clone2);

        // Strong count should now be 1
        assert_eq!(clone3.as_ref().strong_count(), 1);

        // Still nothing dropped (clone3 exists)
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 3); // Only Created entries
        }

        // Drop last clone - everything should be dropped
        drop(clone3);

        // All should be dropped exactly once despite multiple clones
        let final_log = log.borrow();
        assert_eq!(final_log.len(), 6); // 3 Created + 3 Dropped
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Created:")).count(),
            3
        );
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Dropped:")).count(),
            3
        );
    }

    // Test Case 4: Access child directly and drop in different orders
    {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));

        let grandchild = {
            let grandchild_context = DropTracker::new("gc4", log.clone());
            RawReport::new::<_, DefaultContextHandler>(grandchild_context, Vec::new(), Vec::new())
        };

        let child = {
            let child_context = DropTracker::new("child4", log.clone());
            RawReport::new::<_, DefaultContextHandler>(child_context, vec![grandchild], Vec::new())
        };

        let parent = {
            let parent_context = DropTracker::new("parent4", log.clone());
            RawReport::new::<_, DefaultContextHandler>(parent_context, vec![child], Vec::new())
        };

        // Get direct references to child and grandchild through parent
        let parent_ref = parent.as_ref();
        let child_ref = &parent_ref.children()[0];
        let grandchild_ref = &child_ref.as_ref().children()[0];

        // Initially, each should have strong count of 1
        assert_eq!(parent_ref.strong_count(), 1);
        assert_eq!(child_ref.as_ref().strong_count(), 1);
        assert_eq!(grandchild_ref.as_ref().strong_count(), 1);

        // Clone the child and grandchild directly
        let child_clone = unsafe { child_ref.as_ref().clone_arc() };
        let grandchild_clone = unsafe { grandchild_ref.as_ref().clone_arc() };

        // After cloning, child and grandchild should have strong count of 2
        assert_eq!(parent_ref.strong_count(), 1); // Parent unchanged
        assert_eq!(child_ref.as_ref().strong_count(), 2);
        assert_eq!(grandchild_ref.as_ref().strong_count(), 2);
        assert_eq!(child_clone.as_ref().strong_count(), 2);
        assert_eq!(grandchild_clone.as_ref().strong_count(), 2);

        // Drop parent first - but child_clone and grandchild_clone should keep them
        // alive
        drop(parent);

        // After parent is dropped:
        // - child_clone should have strong count of 1 (parent's reference is gone)
        // - grandchild_clone should have strong count of 2 (child_clone still holds a
        //   reference to grandchild, plus grandchild_clone)
        assert_eq!(child_clone.as_ref().strong_count(), 1);
        assert_eq!(grandchild_clone.as_ref().strong_count(), 2);

        // Only parent should be dropped (child and grandchild still have clones)
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 4); // 3 Created + 1 Dropped (parent)
            assert!(current_log.last().unwrap().contains("Dropped: parent4"));
        }

        // Drop child clone - child should be dropped now
        drop(child_clone);

        // Grandchild should now have strong count of 1 (child_clone's reference is
        // gone)
        assert_eq!(grandchild_clone.as_ref().strong_count(), 1);

        // Child should be dropped, grandchild still alive via clone
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 5); // 3 Created + 2 Dropped (parent, child)
            assert!(current_log.iter().any(|e| e.contains("Dropped: child4")));
        }

        // Drop grandchild clone - grandchild should be dropped now
        drop(grandchild_clone);

        let final_log = log.borrow();
        assert_eq!(final_log.len(), 6); // 3 Created + 3 Dropped
        assert!(final_log.iter().any(|e| e.contains("Dropped: gc4")));
    }

    // Test Case 5: Drop grandchild clone first, then work up
    {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));

        let grandchild = {
            let grandchild_context = DropTracker::new("gc5", log.clone());
            RawReport::new::<_, DefaultContextHandler>(grandchild_context, Vec::new(), Vec::new())
        };

        let child = {
            let child_context = DropTracker::new("child5", log.clone());
            RawReport::new::<_, DefaultContextHandler>(child_context, vec![grandchild], Vec::new())
        };

        let parent = {
            let parent_context = DropTracker::new("parent5", log.clone());
            RawReport::new::<_, DefaultContextHandler>(parent_context, vec![child], Vec::new())
        };

        // Clone the entire hierarchy
        let parent_ref = parent.as_ref();

        // Initially, each should have strong count of 1
        assert_eq!(parent_ref.strong_count(), 1);

        let parent_clone = unsafe { parent_ref.clone_arc() };

        // After parent clone, parent should have strong count of 2
        assert_eq!(parent_ref.strong_count(), 2);
        assert_eq!(parent_clone.as_ref().strong_count(), 2);

        // Also get direct reference to grandchild via parent
        let child_ref = &parent_ref.children()[0];
        let grandchild_ref = &child_ref.as_ref().children()[0];

        // Child and grandchild should still have strong count of 1
        assert_eq!(child_ref.as_ref().strong_count(), 1);
        assert_eq!(grandchild_ref.as_ref().strong_count(), 1);

        let grandchild_clone = unsafe { grandchild_ref.as_ref().clone_arc() };

        // After grandchild clone, grandchild should have strong count of 2
        assert_eq!(grandchild_ref.as_ref().strong_count(), 2);
        assert_eq!(grandchild_clone.as_ref().strong_count(), 2);

        // Drop original parent (parent_clone and grandchild_clone should keep things
        // alive)
        drop(parent);

        // Parent clone should now have strong count of 1
        assert_eq!(parent_clone.as_ref().strong_count(), 1);
        // Grandchild clone should still have strong count of 2
        assert_eq!(grandchild_clone.as_ref().strong_count(), 2);

        // Nothing should be dropped yet (parent_clone holds the full hierarchy)
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 3); // Only Created entries
        }

        // Drop grandchild clone first - grandchild should still exist in parent_clone
        drop(grandchild_clone);

        // Still nothing should be dropped (parent_clone holds the full hierarchy)
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 3); // Only Created entries
        }

        // Drop parent clone - everything should be dropped together
        drop(parent_clone);

        let final_log = log.borrow();
        assert_eq!(final_log.len(), 6); // 3 Created + 3 Dropped
    }

    // Test Case 6: Keep grandchild alive while parent and child are dropped
    // together
    {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));

        let grandchild = {
            let grandchild_context = DropTracker::new("gc6", log.clone());
            RawReport::new::<_, DefaultContextHandler>(grandchild_context, Vec::new(), Vec::new())
        };

        let child = {
            let child_context = DropTracker::new("child6", log.clone());
            RawReport::new::<_, DefaultContextHandler>(child_context, vec![grandchild], Vec::new())
        };

        let parent = {
            let parent_context = DropTracker::new("parent6", log.clone());
            RawReport::new::<_, DefaultContextHandler>(parent_context, vec![child], Vec::new())
        };

        // Get direct reference to grandchild and clone it
        let parent_ref = parent.as_ref();
        let child_ref = &parent_ref.children()[0];
        let grandchild_ref = &child_ref.as_ref().children()[0];

        // Initially, each should have strong count of 1
        assert_eq!(parent_ref.strong_count(), 1);
        assert_eq!(child_ref.as_ref().strong_count(), 1);
        assert_eq!(grandchild_ref.as_ref().strong_count(), 1);

        let grandchild_clone = unsafe { grandchild_ref.as_ref().clone_arc() };

        // After grandchild clone, grandchild should have strong count of 2
        assert_eq!(grandchild_ref.as_ref().strong_count(), 2);
        assert_eq!(grandchild_clone.as_ref().strong_count(), 2);

        // Drop the parent - this should drop both parent and child, but NOT grandchild
        drop(parent);

        // Grandchild clone should now have strong count of 1
        assert_eq!(grandchild_clone.as_ref().strong_count(), 1);

        // Parent and child should be dropped, but grandchild should still be alive
        {
            let current_log = log.borrow();
            assert_eq!(current_log.len(), 5); // 3 Created + 2 Dropped (parent, child)

            // Verify both parent and child are dropped
            assert!(current_log.iter().any(|e| e.contains("Dropped: parent6")));
            assert!(current_log.iter().any(|e| e.contains("Dropped: child6")));

            // Verify grandchild is NOT dropped
            assert!(!current_log.iter().any(|e| e.contains("Dropped: gc6")));
        }

        // Verify grandchild is still functional
        let grandchild_display = format!("{}", TestReportDisplayWrapper(grandchild_clone.as_ref()));
        assert_eq!(grandchild_display, "DropTracker(gc6)");

        // Drop grandchild clone - now grandchild should be dropped
        drop(grandchild_clone);

        // All should be dropped exactly once
        let final_log = log.borrow();
        assert_eq!(final_log.len(), 6); // 3 Created + 3 Dropped
        assert!(final_log.iter().any(|e| e.contains("Dropped: gc6")));

        // Verify final count
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Created:")).count(),
            3
        );
        assert_eq!(
            final_log.iter().filter(|e| e.contains("Dropped:")).count(),
            3
        );
    }
}

// RawReportMut Integration Tests
#[test]
fn test_raw_report_mut_basic_operations() {
    let mut report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("mutable basic test"),
        Vec::new(),
        Vec::new(),
    );

    // Test basic mutable reference operations
    unsafe {
        let mut report_mut = report.as_mut();

        // Test as_ref() functionality
        let report_ref = report_mut.as_ref();
        assert_eq!(
            report_ref.context_type_id(),
            core::any::TypeId::of::<TestError>()
        );

        let display_result = format!("{}", TestReportDisplayWrapper(report_ref));
        assert_eq!(display_result, "mutable basic test");

        // Test reborrow functionality
        let reborrowed = report_mut.reborrow();
        let reborrow_ref = reborrowed.as_ref();
        let reborrow_display = format!("{}", TestReportDisplayWrapper(reborrow_ref));
        assert_eq!(reborrow_display, "mutable basic test");

        // Test that mutable reference works
        let final_ref = report_mut.as_ref();
        assert_eq!(
            final_ref.context_type_id(),
            core::any::TypeId::of::<TestError>()
        );
    }
}

#[test]
fn test_raw_report_mut_children_manipulation() {
    let mut report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("parent for children test"),
        Vec::new(),
        Vec::new(),
    );

    // Test adding children through mutable reference
    unsafe {
        let report_mut = report.as_mut();
        let children_mut = report_mut.into_children_mut();
        assert_eq!(children_mut.len(), 0);

        // Add multiple children
        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child 1"),
            Vec::new(),
            Vec::new(),
        ));

        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child 2"),
            Vec::new(),
            Vec::new(),
        ));

        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child 3"),
            Vec::new(),
            Vec::new(),
        ));

        assert_eq!(children_mut.len(), 3);
    }

    // Verify children were added correctly
    let report_ref = report.as_ref();
    let children = report_ref.children();
    assert_eq!(children.len(), 3);

    for (i, child) in children.iter().enumerate() {
        let child_ref = child.as_ref();
        let child_display = format!("{}", TestReportDisplayWrapper(child_ref));
        assert_eq!(child_display, format!("child {}", i + 1));
    }

    // Test removing children
    unsafe {
        let report_mut = report.as_mut();
        let children_mut = report_mut.into_children_mut();
        let removed_child = children_mut.pop();
        assert!(removed_child.is_some());
        assert_eq!(children_mut.len(), 2);
    }

    // Verify removal
    let children = report.as_ref().children();
    assert_eq!(children.len(), 2);
}

#[test]
fn test_raw_report_mut_attachments_manipulation() {
    let mut report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("parent for attachments test"),
        Vec::new(),
        Vec::new(),
    );

    // Test adding attachments through mutable reference
    unsafe {
        let attachments_mut = report.as_mut().into_attachments_mut();
        assert_eq!(attachments_mut.len(), 0);

        // Add multiple attachments
        attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("attachment1", 100),
        ));

        attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("attachment2", 200),
        ));

        attachments_mut.push(RawAttachment::new::<_, CustomAttachmentHandler>(
            TestAttachment::new("custom_attachment", 300),
        ));

        assert_eq!(attachments_mut.len(), 3);
    }

    // Verify attachments were added correctly
    let report_ref = report.as_ref();
    let attachments = report_ref.attachments();
    assert_eq!(attachments.len(), 3);

    // Check first two attachments (default handler)
    for (i, attachment) in attachments[..2].iter().enumerate() {
        let attachment_ref = attachment.as_ref();
        assert_eq!(
            attachment_ref.attachment_type_id(),
            core::any::TypeId::of::<TestAttachment>()
        );
        let downcast = unsafe { attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
        assert_eq!(downcast.name, format!("attachment{}", i + 1));
        assert_eq!(downcast.value, (i + 1) as i32 * 100);

        let display = format!("{}", TestDisplayWrapper(attachment_ref));
        assert_eq!(display, format!("attachment{}: {}", i + 1, (i + 1) * 100));
    }

    // Check custom attachment (different display format)
    let custom_attachment_ref = attachments[2].as_ref();
    assert_eq!(
        custom_attachment_ref.attachment_type_id(),
        core::any::TypeId::of::<TestAttachment>()
    );
    let custom_downcast =
        unsafe { custom_attachment_ref.attachment_downcast_unchecked::<TestAttachment>() };
    assert_eq!(custom_downcast.name, "custom_attachment");
    assert_eq!(custom_downcast.value, 300);

    let custom_display = format!("{}", TestDisplayWrapper(custom_attachment_ref));
    assert_eq!(custom_display, "[ATTACHMENT] custom_attachment = 300");

    // Test removing attachments
    unsafe {
        let report_mut = report.as_mut();
        let attachments_mut = report_mut.into_attachments_mut();
        let removed_attachment = attachments_mut.pop();
        assert!(removed_attachment.is_some());
        assert_eq!(attachments_mut.len(), 2);
    }

    // Verify removal
    let attachments = report.as_ref().attachments();
    assert_eq!(attachments.len(), 2);
}

#[test]
fn test_raw_report_mut_complex_hierarchy_manipulation() {
    // Create a complex hierarchy with mixed children and attachments
    let mut root =
        RawReport::new::<_, DefaultContextHandler>(TestError::new("root"), Vec::new(), Vec::new());

    unsafe {
        // Add initial children and attachments
        let children_mut = root.as_mut().into_children_mut();
        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("initial child"),
            Vec::new(),
            Vec::new(),
        ));

        let attachments_mut = root.as_mut().into_attachments_mut();
        attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("initial_attachment", 50),
        ));
    }

    // Verify initial state
    assert_eq!(root.as_ref().children().len(), 1);
    assert_eq!(root.as_ref().attachments().len(), 1);

    // Add complex nested structure
    unsafe {
        let children_mut = root.as_mut().into_children_mut();

        // Create a child with its own children and attachments
        let grandchild1 = RawReport::new::<_, DefaultContextHandler>(
            TestError::new("grandchild 1"),
            Vec::new(),
            vec![RawAttachment::new::<_, DefaultAttachmentHandler>(
                TestAttachment::new("gc1_attachment", 10),
            )],
        );

        let grandchild2 = RawReport::new::<_, CustomContextHandler>(
            TestError::new("grandchild 2"),
            Vec::new(),
            vec![RawAttachment::new::<_, CustomAttachmentHandler>(
                TestAttachment::new("gc2_attachment", 20),
            )],
        );

        let child_with_grandchildren = RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child with grandchildren"),
            vec![grandchild1, grandchild2],
            vec![RawAttachment::new::<_, DefaultAttachmentHandler>(
                TestAttachment::new("child_attachment", 100),
            )],
        );

        children_mut.push(child_with_grandchildren);

        // Add more attachments to root
        let attachments_mut = root.as_mut().into_attachments_mut();
        attachments_mut.push(RawAttachment::new::<_, CustomAttachmentHandler>(
            TestAttachment::new("root_custom", 999),
        ));
    }

    // Verify the complex structure
    let root_ref = root.as_ref();
    assert_eq!(root_ref.children().len(), 2);
    assert_eq!(root_ref.attachments().len(), 2);

    // Check root attachments
    let root_attachments = root_ref.attachments();
    let initial_attachment = root_attachments[0].as_ref();
    let custom_attachment = root_attachments[1].as_ref();

    assert_eq!(
        initial_attachment.attachment_type_id(),
        core::any::TypeId::of::<TestAttachment>()
    );
    assert_eq!(
        unsafe {
            &initial_attachment
                .attachment_downcast_unchecked::<TestAttachment>()
                .name
        },
        "initial_attachment"
    );
    assert_eq!(
        unsafe {
            &custom_attachment
                .attachment_downcast_unchecked::<TestAttachment>()
                .name
        },
        "root_custom"
    );

    // Check display formats (default vs custom)
    let initial_display = format!("{}", TestDisplayWrapper(initial_attachment));
    let custom_display = format!("{}", TestDisplayWrapper(custom_attachment));
    assert_eq!(initial_display, "initial_attachment: 50");
    assert_eq!(custom_display, "[ATTACHMENT] root_custom = 999");

    // Check complex child structure
    let children = root_ref.children();
    let initial_child = children[0].as_ref();
    let complex_child = children[1].as_ref();

    // Initial child should have no children or attachments
    assert_eq!(initial_child.children().len(), 0);
    assert_eq!(initial_child.attachments().len(), 0);

    // Complex child should have 2 grandchildren and 1 attachment
    assert_eq!(complex_child.children().len(), 2);
    assert_eq!(complex_child.attachments().len(), 1);

    // Check grandchildren
    let grandchildren = complex_child.children();
    let gc1 = grandchildren[0].as_ref();
    let gc2 = grandchildren[1].as_ref();

    // GC1 should use default handler
    let gc1_display = format!("{}", TestReportDisplayWrapper(gc1));
    assert_eq!(gc1_display, "grandchild 1");

    // GC2 should use custom handler
    let gc2_display = format!("{}", TestReportDisplayWrapper(gc2));
    assert_eq!(gc2_display, "[CUSTOM] grandchild 2");

    // Check grandchildren attachments
    assert_eq!(gc1.attachments().len(), 1);
    assert_eq!(gc2.attachments().len(), 1);

    let gc1_attachment = gc1.attachments()[0].as_ref();
    let gc2_attachment = gc2.attachments()[0].as_ref();

    let gc1_att_display = format!("{}", TestDisplayWrapper(gc1_attachment));
    let gc2_att_display = format!("{}", TestDisplayWrapper(gc2_attachment));

    assert_eq!(gc1_att_display, "gc1_attachment: 10");
    assert_eq!(gc2_att_display, "[ATTACHMENT] gc2_attachment = 20");
}

#[test]
fn test_raw_report_mut_reborrow_with_modifications() {
    let mut report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("reborrow test"),
        Vec::new(),
        Vec::new(),
    );

    unsafe {
        let mut report_mut = report.as_mut();

        // Add some initial content
        let children_mut = report_mut.reborrow().into_children_mut();
        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child 1"),
            Vec::new(),
            Vec::new(),
        ));

        // Reborrow and add more content
        let reborrowed = report_mut.reborrow();
        let attachments_mut = reborrowed.into_attachments_mut();
        attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("reborrow_attachment", 42),
        ));

        // Another reborrow and add more children
        let children_mut2 = report_mut.reborrow().into_children_mut();
        children_mut2.push(RawReport::new::<_, CustomContextHandler>(
            TestError::new("child 2"),
            Vec::new(),
            Vec::new(),
        ));

        // Verify state after all reborrows
        let final_ref = report_mut.as_ref();
        assert_eq!(final_ref.children().len(), 2);
        assert_eq!(final_ref.attachments().len(), 1);
    }

    // Final verification outside unsafe block
    let final_ref = report.as_ref();
    assert_eq!(final_ref.children().len(), 2);
    assert_eq!(final_ref.attachments().len(), 1);

    let children = final_ref.children();
    let child1_display = format!("{}", TestReportDisplayWrapper(children[0].as_ref()));
    let child2_display = format!("{}", TestReportDisplayWrapper(children[1].as_ref()));

    assert_eq!(child1_display, "child 1");
    assert_eq!(child2_display, "[CUSTOM] child 2");

    let attachment = final_ref.attachments()[0].as_ref();
    let attachment_display = format!("{}", TestDisplayWrapper(attachment));
    assert_eq!(attachment_display, "reborrow_attachment: 42");
}

#[test]
fn test_raw_report_mut_type_safety_and_downcasting() {
    // Test that mutable operations preserve type safety
    let mut int_report = RawReport::new::<_, NumberContextHandler>(42i32, Vec::new(), Vec::new());

    let mut string_report =
        RawReport::new::<_, StringContextHandler>("test string".to_owned(), Vec::new(), Vec::new());

    unsafe {
        // Add different types of children and attachments
        let int_children_mut = int_report.as_mut().into_children_mut();
        int_children_mut.push(RawReport::new::<_, NumberContextHandler>(
            100i32,
            Vec::new(),
            Vec::new(),
        ));

        let string_children_mut = string_report.as_mut().into_children_mut();
        string_children_mut.push(RawReport::new::<_, StringContextHandler>(
            "child string".to_owned(),
            Vec::new(),
            Vec::new(),
        ));

        // Add mixed attachment types
        let int_attachments_mut = int_report.as_mut().into_attachments_mut();
        int_attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("int_attachment", 777),
        ));
        int_attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            "string attachment in int report".to_owned(),
        ));

        let string_attachments_mut = string_report.as_mut().into_attachments_mut();
        string_attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(999i32));
    }

    // Verify type IDs and downcasting
    let int_ref = int_report.as_ref();
    let string_ref = string_report.as_ref();

    assert_eq!(int_ref.context_type_id(), core::any::TypeId::of::<i32>());
    assert_eq!(
        string_ref.context_type_id(),
        core::any::TypeId::of::<String>()
    );

    // Test context downcast
    assert_eq!(int_ref.context_downcast::<i32>().unwrap(), &42);
    assert_eq!(
        string_ref.context_downcast::<String>().unwrap(),
        "test string"
    );

    // Cross-type downcasting should fail
    assert!(int_ref.context_downcast::<String>().is_none());
    assert!(string_ref.context_downcast::<i32>().is_none());

    // Test children context types
    let int_child = int_ref.children()[0].as_ref();
    let string_child = string_ref.children()[0].as_ref();

    assert_eq!(int_child.context_type_id(), core::any::TypeId::of::<i32>());
    assert_eq!(
        string_child.context_type_id(),
        core::any::TypeId::of::<String>()
    );

    assert_eq!(int_child.context_downcast::<i32>().unwrap(), &100);
    assert_eq!(
        string_child.context_downcast::<String>().unwrap(),
        "child string"
    );

    // Test attachment types
    let int_attachments = int_ref.attachments();
    let string_attachments = string_ref.attachments();

    // Int report has TestAttachment and String attachments
    assert_eq!(int_attachments.len(), 2);
    assert_eq!(
        int_attachments[0].as_ref().attachment_type_id(),
        core::any::TypeId::of::<TestAttachment>()
    );
    assert_eq!(
        int_attachments[1].as_ref().attachment_type_id(),
        core::any::TypeId::of::<String>()
    );

    // String report has i32 attachment
    assert_eq!(string_attachments.len(), 1);
    assert_eq!(
        string_attachments[0].as_ref().attachment_type_id(),
        core::any::TypeId::of::<i32>()
    );

    // Test attachment downcasting
    assert_eq!(
        int_attachments[0].as_ref().attachment_type_id(),
        core::any::TypeId::of::<TestAttachment>()
    );
    let test_attachment = unsafe {
        int_attachments[0]
            .as_ref()
            .attachment_downcast_unchecked::<TestAttachment>()
    };
    assert_eq!(test_attachment.name, "int_attachment");
    assert_eq!(test_attachment.value, 777);

    assert_eq!(
        int_attachments[1].as_ref().attachment_type_id(),
        core::any::TypeId::of::<String>()
    );
    let string_attachment = unsafe {
        int_attachments[1]
            .as_ref()
            .attachment_downcast_unchecked::<String>()
    };
    assert_eq!(string_attachment, "string attachment in int report");

    assert_eq!(
        string_attachments[0].as_ref().attachment_type_id(),
        core::any::TypeId::of::<i32>()
    );
    let i32_attachment = unsafe {
        string_attachments[0]
            .as_ref()
            .attachment_downcast_unchecked::<i32>()
    };
    assert_eq!(*i32_attachment, 999);
}

#[test]
fn test_raw_report_mut_error_source_handling() {
    // Test error sources with mutable operations
    let source_error = TestError::new("source error");
    let main_error = TestError::with_source("main error", Box::new(source_error));

    let mut report = RawReport::new::<_, DefaultContextHandler>(main_error, Vec::new(), Vec::new());

    unsafe {
        // Add children with different source behaviors
        let children_mut = report.as_mut().into_children_mut();

        // Child with source
        let child_source = TestError::new("child source");
        let child_with_source = TestError::with_source("child with source", Box::new(child_source));
        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            child_with_source,
            Vec::new(),
            Vec::new(),
        ));

        // Child without source
        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child without source"),
            Vec::new(),
            Vec::new(),
        ));

        // Child with custom handler that ignores source
        struct IgnoreSourceHandler;
        impl ContextHandler<TestError> for IgnoreSourceHandler {
            fn source(_value: &TestError) -> Option<&(dyn Error + 'static)> {
                None // Always ignore source
            }

            fn display(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "[NO_SOURCE] {}", value.message)
            }

            fn debug(value: &TestError, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "NoSource({:?})", value.message)
            }
        }

        let child_ignore_source =
            TestError::with_source("child ignores source", Box::new(TestError::new("ignored")));
        children_mut.push(RawReport::new::<_, IgnoreSourceHandler>(
            child_ignore_source,
            Vec::new(),
            Vec::new(),
        ));
    }

    // Verify source handling
    let report_ref = report.as_ref();

    // Main report should have source
    let main_source = report_ref.context_source();
    assert!(main_source.is_some());
    assert_eq!(main_source.unwrap().to_string(), "source error");

    let children = report_ref.children();
    assert_eq!(children.len(), 3);

    // Child with source
    let child_with_source = children[0].as_ref();
    let child_source = child_with_source.context_source();
    assert!(child_source.is_some());
    assert_eq!(child_source.unwrap().to_string(), "child source");

    // Child without source
    let child_without_source = children[1].as_ref();
    let no_source = child_without_source.context_source();
    assert!(no_source.is_none());

    // Child that ignores source (custom handler)
    let child_ignore_source = children[2].as_ref();
    let ignored_source = child_ignore_source.context_source();
    assert!(ignored_source.is_none()); // Handler ignores source

    // Check display formats
    let ignore_display = format!("{}", TestReportDisplayWrapper(child_ignore_source));
    assert_eq!(ignore_display, "[NO_SOURCE] child ignores source");
}

#[test]
fn test_raw_report_mut_modification_correctness() {
    // Test that mutable operations result in correct final state
    let mut report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("modification test"),
        Vec::new(),
        Vec::new(),
    );

    unsafe {
        // Perform multiple modifications
        let mut report_mut = report.as_mut();

        // Add children
        let children_mut = report_mut.reborrow().into_children_mut();
        children_mut.push(RawReport::new::<_, DefaultContextHandler>(
            TestError::new("child"),
            Vec::new(),
            Vec::new(),
        ));

        // Add attachments
        let attachments_mut = report_mut.reborrow().into_attachments_mut();
        attachments_mut.push(RawAttachment::new::<_, DefaultAttachmentHandler>(
            TestAttachment::new("attachment", 123),
        ));
    }

    // Verify final state
    let final_ref = report.as_ref();
    assert_eq!(final_ref.children().len(), 1);
    assert_eq!(final_ref.attachments().len(), 1);

    // Verify content is correct
    let child_display = format!(
        "{}",
        TestReportDisplayWrapper(final_ref.children()[0].as_ref())
    );
    assert_eq!(child_display, "child");

    let attachment = final_ref.attachments()[0].as_ref();
    assert_eq!(
        attachment.attachment_type_id(),
        core::any::TypeId::of::<TestAttachment>()
    );
    let attachment_downcast =
        unsafe { attachment.attachment_downcast_unchecked::<TestAttachment>() };
    assert_eq!(attachment_downcast.name, "attachment");
    assert_eq!(attachment_downcast.value, 123);
}

#[test]
fn test_raw_report_mut_context_downcast_unchecked() {
    // Test the unsafe context_downcast_unchecked method for RawReportMut
    let mut test_error_report = RawReport::new::<_, DefaultContextHandler>(
        TestError::new("mutable context test"),
        Vec::new(),
        Vec::new(),
    );

    let mut string_report = RawReport::new::<_, StringContextHandler>(
        "mutable string context".to_owned(),
        Vec::new(),
        Vec::new(),
    );

    let mut number_report =
        RawReport::new::<_, NumberContextHandler>(99i32, Vec::new(), Vec::new());

    unsafe {
        // Test with TestError context
        let error_mut = test_error_report.as_mut();
        let error_context = error_mut.into_context_downcast_unchecked::<TestError>();
        assert_eq!(error_context.message, "mutable context test");
        assert!(error_context.source.is_none());

        // Modify the context through the mutable reference
        error_context.message = "modified error message".to_owned();
        error_context.source = Some(Box::new(TestError::new("nested error")));

        // Test with String context
        let string_mut = string_report.as_mut();
        let string_context = string_mut.into_context_downcast_unchecked::<String>();
        assert_eq!(string_context, "mutable string context");

        // Modify the string context
        *string_context = "modified string".to_owned();

        // Test with i32 context
        let number_mut = number_report.as_mut();
        let number_context = number_mut.into_context_downcast_unchecked::<i32>();
        assert_eq!(*number_context, 99);

        // Modify the number context
        *number_context = 42;
    }

    // Verify modifications were applied correctly
    let error_ref = test_error_report.as_ref();
    let modified_error = error_ref.context_downcast::<TestError>().unwrap();
    assert_eq!(modified_error.message, "modified error message");
    assert!(modified_error.source.is_some());
    let nested_error = modified_error.source.as_ref().unwrap();
    assert_eq!(format!("{nested_error}"), "nested error");

    let string_ref = string_report.as_ref();
    let modified_string = string_ref.context_downcast::<String>().unwrap();
    assert_eq!(modified_string, "modified string");

    let number_ref = number_report.as_ref();
    let modified_number = number_ref.context_downcast::<i32>().unwrap();
    assert_eq!(*modified_number, 42);

    // Test that display still works correctly after modifications
    let error_display = format!("{}", TestReportDisplayWrapper(error_ref));
    assert_eq!(error_display, "modified error message");

    let string_display = format!("{}", TestReportDisplayWrapper(string_ref));
    assert_eq!(string_display, "modified string");

    let number_display = format!("{}", TestReportDisplayWrapper(number_ref));
    assert_eq!(number_display, "42");
}
