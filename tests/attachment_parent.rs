//! Locks down `AttachmentParent.attachment_index` semantics in
//! `DefaultReportFormatter`.
//!
//! The index reported to an `AttachmentFormatterHook` must reflect the
//! attachment's position in `report.attachments()`, not the position the
//! formatter ends up displaying it at after priority-sorting.

use rootcause::{
    handlers::{AttachmentFormattingPlacement, AttachmentFormattingStyle, FormattingFunction},
    hooks::{
        Hooks,
        attachment_formatter::{AttachmentFormatterHook, AttachmentParent},
    },
    prelude::*,
    report_attachment::ReportAttachmentRef,
};

#[derive(Debug)]
struct Probe {
    tag: &'static str,
    priority: i32,
}

impl core::fmt::Display for Probe {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.tag)
    }
}

struct ProbeFormatter;

impl AttachmentFormatterHook<Probe> for ProbeFormatter {
    fn display(
        &self,
        attachment: ReportAttachmentRef<'_, Probe>,
        parent: Option<AttachmentParent<'_>>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        let tag = attachment.inner().tag;
        match parent {
            Some(parent) => write!(f, "{tag}@{}", parent.attachment_index),
            None => write!(f, "{tag}@NONE"),
        }
    }

    fn preferred_formatting_style(
        &self,
        attachment: ReportAttachmentRef<'_, Probe>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Inline,
            function: FormattingFunction::Display,
            priority: attachment.inner().priority,
        }
    }
}

#[test]
fn attachment_index_is_pre_sort_position() {
    // `new_without_locations` keeps attachment indices simple: 0, 1, 2 — no
    // auto-Location attachment slipping in at index 0.
    Hooks::new_without_locations()
        .attachment_formatter::<Probe, _>(ProbeFormatter)
        .install()
        .expect("hooks should not already be installed");

    // Priorities chosen so the display order differs from the insertion order:
    // sorted descending → "second" (10), "first" (0), "third" (-10).
    let report: Report = report!("outer")
        .attach(Probe {
            tag: "first",
            priority: 0,
        })
        .attach(Probe {
            tag: "second",
            priority: 10,
        })
        .attach(Probe {
            tag: "third",
            priority: -10,
        });

    let rendered = format!("{report}");

    // Core invariant: each probe sees its ORIGINAL index, not its display index.
    assert!(
        rendered.contains("first@0"),
        "expected `first@0` (original index 0) in output:\n{rendered}"
    );
    assert!(
        rendered.contains("second@1"),
        "expected `second@1` (original index 1) in output:\n{rendered}"
    );
    assert!(
        rendered.contains("third@2"),
        "expected `third@2` (original index 2) in output:\n{rendered}"
    );

    // Sanity: confirm the formatter actually re-sorted. Without this check the
    // index-equality assertions above would still pass if sorting were a no-op,
    // which would mean we weren't really testing the pre-sort property.
    let pos = |needle: &str| {
        rendered
            .find(needle)
            .unwrap_or_else(|| panic!("`{needle}` not found in:\n{rendered}"))
    };
    let second = pos("second@1");
    let first = pos("first@0");
    let third = pos("third@2");
    assert!(
        second < first && first < third,
        "expected priority-sorted display order second,first,third but got:\n{rendered}"
    );

    // Bonus: `format_inner()` (no parent) reports `NONE`.
    let isolated = format!(
        "{}",
        rootcause::report_attachment::ReportAttachment::new_local(Probe {
            tag: "iso",
            priority: 0,
        })
        .as_ref()
        .format_inner()
    );
    assert_eq!(isolated, "iso@NONE");
}
