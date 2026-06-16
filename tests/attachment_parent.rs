//! Locks down `AttachmentParent.attachment_index` semantics in
//! `DefaultReportFormatter`.
//!
//! The index reported to an `AttachmentFormatterHook` must reflect the
//! attachment's position in `report.attachments()`, not the position the
//! formatter ends up displaying it at after priority-sorting, and this must
//! hold for both `Inline` and `Appendix` placements.

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

#[derive(Debug)]
struct AppendixProbe {
    tag: &'static str,
    priority: i32,
}

impl core::fmt::Display for AppendixProbe {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.tag)
    }
}

struct AppendixProbeFormatter;

impl AttachmentFormatterHook<AppendixProbe> for AppendixProbeFormatter {
    fn display(
        &self,
        attachment: ReportAttachmentRef<'_, AppendixProbe>,
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
        attachment: ReportAttachmentRef<'_, AppendixProbe>,
        _report_formatting_function: FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: AttachmentFormattingPlacement::Appendix {
                appendix_name: "Diagnostics",
            },
            function: FormattingFunction::Display,
            priority: attachment.inner().priority,
        }
    }
}

#[test]
fn attachment_index_is_pre_sort_position() {
    // `new_without_locations` keeps attachment indices simple: 0, 1, 2, ... — no
    // auto-Location attachment slipping in at index 0.
    Hooks::new_without_locations()
        .attachment_formatter::<Probe, _>(ProbeFormatter)
        .attachment_formatter::<AppendixProbe, _>(AppendixProbeFormatter)
        .install()
        .expect("hooks should not already be installed");

    // Priorities chosen so the display order differs from the insertion order:
    // sorted descending → "second" (10), "appendix_first" (5), "first" (0),
    // "appendix_second" (-2), "third" (-10).
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
        })
        .attach(AppendixProbe {
            tag: "appendix_first",
            priority: 5,
        })
        .attach(AppendixProbe {
            tag: "appendix_second",
            priority: -2,
        });

    let rendered = format!("{report}");

    // Core invariant: each probe sees its ORIGINAL index, not its display
    // index. Holds for both Inline and Appendix placements.
    for needle in [
        "first@0",
        "second@1",
        "third@2",
        "appendix_first@3",
        "appendix_second@4",
    ] {
        assert!(
            rendered.contains(needle),
            "expected `{needle}` in output:\n{rendered}"
        );
    }

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
        "expected priority-sorted inline display order second,first,third but got:\n{rendered}"
    );
    let appendix_first = pos("appendix_first@3");
    let appendix_second = pos("appendix_second@4");
    assert!(
        appendix_first < appendix_second,
        "expected priority-sorted appendix display order appendix_first,appendix_second but got:\n{rendered}"
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
