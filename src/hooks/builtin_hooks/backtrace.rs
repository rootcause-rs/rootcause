//! Stack backtrace attachment collector.
//!
//! This module provides functionality to automatically capture and attach stack
//! backtraces to reports when they are created. This is useful for debugging to
//! see the call stack that led to an error.

use alloc::borrow::Cow;
use core::{fmt, panic::Location};
use std::{path::PathBuf, sync::OnceLock};

use backtrace::BytesOrWideString;
use rootcause_internals::handlers::{
    AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
};

use crate::{hooks::report_creation::ReportCreationHook, report_attachment::ReportAttachment};

/// Stack backtrace information.
///
/// Contains a collection of stack frames representing the call stack
/// at the point where a report was created.
pub struct Backtrace {
    /// The entries in the backtrace, ordered from most recent to oldest.
    pub entries: Vec<BacktraceEntry>,
    /// Total number of frames that were omitted due to filtering.
    pub total_omitted_frames: usize,
}

/// A single entry in a stack backtrace.
pub enum BacktraceEntry {
    /// A normal stack frame.
    Frame(Frame),
    /// A group of omitted frames from a specific crate.
    OmittedFrames {
        /// Number of omitted frames.
        count: usize,
        /// The name of the crate whose frames were omitted.
        skipped_crate: &'static str,
    },
}

/// A single stack frame in a backtrace.
///
/// Represents one function call in the call stack, including symbol information
/// and source location if available.
pub struct Frame {
    /// The demangled symbol name for this frame.
    pub sym_demangled: String,
    /// File path information for this frame, if available.
    pub frame_path: Option<FramePath>,
    /// Line number in the source file, if available.
    pub lineno: Option<u32>,
}

/// File path information for a stack frame.
///
/// Contains the raw path and processed components for better display
/// formatting.
#[derive(Debug)]
pub struct FramePath {
    /// The raw file path from the debug information.
    pub raw_path: PathBuf,
    /// The crate name if detected from the path.
    pub crate_name: Option<Cow<'static, str>>,
    /// Common path prefix information for shortening display.
    pub split_path: Option<FramePrefix>,
}

/// A common prefix for a frame path.
#[derive(Debug)]
pub struct FramePrefix {
    /// The kind of prefix used to identify this prefix (e.g., "RUST_SRC").
    pub prefix_kind: &'static str,
    /// The full prefix path value.
    pub prefix: String,
    /// The full suffix path value.
    pub suffix: String,
}

/// Handler for formatting [`Backtrace`] attachments.
#[derive(Copy, Clone)]
pub struct BacktraceHandler<const SHOW_FULL_PATH: bool>;

impl<const SHOW_FULL_PATH: bool> AttachmentHandler<Backtrace> for BacktraceHandler<SHOW_FULL_PATH> {
    fn display(value: &Backtrace, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &value.entries {
            match entry {
                BacktraceEntry::OmittedFrames {
                    count,
                    skipped_crate,
                } => {
                    writeln!(
                        f,
                        "... omitted {count} frame(s) from crate '{skipped_crate}' ..."
                    )?;
                    continue;
                }
                BacktraceEntry::Frame(frame) => {
                    let sym = frame
                        .sym_demangled
                        .rsplit_once("::")
                        .map_or(frame.sym_demangled.as_str(), |(_, sym)| sym);

                    if let Some(path) = &frame.frame_path {
                        if SHOW_FULL_PATH {
                            write!(f, "{}", path.raw_path.display())?;
                        } else if let Some(split_path) = &path.split_path {
                            write!(f, "[..]/{}", split_path.suffix)?;
                        } else {
                            write!(f, "{}", path.raw_path.display())?;
                        }

                        if let Some(lineno) = frame.lineno {
                            write!(f, ":{lineno}")?;
                        }
                    }

                    writeln!(f, " - {sym}")?;
                }
            }
        }

        if value.total_omitted_frames > 0 {
            writeln!(
                f,
                "note: {} frame(s) omitted. For a complete backtrace, set RUST_BACKTRACE=full.",
                value.total_omitted_frames
            )?;
        }

        Ok(())
    }

    fn debug(value: &Backtrace, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Self::display(value, formatter)
    }

    fn preferred_formatting_style(
        backtrace: &Backtrace,
        _report_formatting_function: rootcause_internals::handlers::FormattingFunction,
    ) -> AttachmentFormattingStyle {
        AttachmentFormattingStyle {
            placement: if backtrace.entries.is_empty() {
                AttachmentFormattingPlacement::Hidden
            } else {
                AttachmentFormattingPlacement::InlineWithHeader {
                    header: "Backtrace",
                }
            },
            priority: 10,
            ..Default::default()
        }
    }
}

/// Attachment collector for capturing stack backtraces.
///
/// When registered as a report creation hook, this collector automatically
/// captures the current stack backtrace and attaches it as a [`Backtrace`]
/// attachment.
#[derive(Copy, Clone)]
pub struct BacktraceCollector {
    /// Whether to show full file paths in the backtrace frames.
    pub show_full_path: bool,
    /// Whether to perform filtering of frames from certain crates.
    pub filter: BacktraceFilter,

    /// If set to true, a backtrace is captured for every report creation,
    /// including reports that have child reports (i.e., reports created with
    /// existing children). If set to false, a backtrace is captured only
    /// for reports created without any children. Reports created without
    /// children always receive a backtrace regardless of this setting.
    pub capture_backtrace_for_reports_with_children: bool,
}

/// Configuration for filtering frames from certain crates in a backtrace.
#[derive(Copy, Clone, Debug)]
pub struct BacktraceFilter {
    /// Set of crate names whose frames should be hidden when they appear
    /// at the beginning of a backtrace.
    pub skipped_initial_crates: &'static [&'static str],
    /// Set of crate names whose frames should be hidden when they appear
    /// in the middle of a backtrace.
    pub skipped_middle_crates: &'static [&'static str],
    /// Set of crate names whose frames should be hidden when they appear
    /// at the end of a backtrace.
    pub skipped_final_crates: &'static [&'static str],
    /// Maximum number of entries to include in the backtrace.
    pub max_entry_count: usize,
}

impl BacktraceCollector {
    /// Creates a new [`BacktraceCollector`] with default settings.
    ///
    /// By default, if the `RUST_BACKTRACE` environment variable is set to
    /// `full`, the collector will filter out frames from common crates and
    /// capture backtraces for all reports. Otherwise, no filtering is applied
    /// and backtraces are only captured for reports without children.
    pub fn new_from_env() -> Self {
        let rust_backtrace_full =
            std::env::var_os("RUST_BACKTRACE").is_some_and(|var| var == "full");
        let leafs_only = std::env::var_os("ROOTCAUSE_BACKTRACE").is_some_and(|var| var == "leafs");
        let capture_backtrace_for_reports_with_children = !leafs_only;

        Self {
            show_full_path: rust_backtrace_full,
            filter: if rust_backtrace_full {
                BacktraceFilter {
                    skipped_initial_crates: &[],
                    skipped_middle_crates: &[],
                    skipped_final_crates: &[],
                    max_entry_count: usize::MAX,
                }
            } else {
                BacktraceFilter::default()
            },
            capture_backtrace_for_reports_with_children,
        }
    }
}

impl Default for BacktraceFilter {
    /// Creates a new [`BacktraceFilter`] with default settings.
    fn default() -> Self {
        Self {
            skipped_initial_crates: &["backtrace", "rootcause", "core", "std", "alloc"],
            skipped_middle_crates: &["tokio"],
            skipped_final_crates: &["std", "core", "alloc", "tokio"],
            max_entry_count: 20,
        }
    }
}

impl ReportCreationHook for BacktraceCollector {
    fn on_local_creation(
        &self,
        mut report: crate::ReportMut<'_, dyn core::any::Any, crate::markers::Local>,
    ) {
        let do_capture =
            self.capture_backtrace_for_reports_with_children || !report.children().is_empty();
        if do_capture && let Some(backtrace) = Backtrace::capture(&self.filter) {
            let attachment = if self.show_full_path {
                ReportAttachment::new_custom::<BacktraceHandler<true>>(backtrace)
            } else {
                ReportAttachment::new_custom::<BacktraceHandler<false>>(backtrace)
            };
            report.attachments_mut().push(attachment.into_dyn_any());
        }
    }

    fn on_sendsync_creation(
        &self,
        mut report: crate::ReportMut<'_, dyn core::any::Any, crate::markers::SendSync>,
    ) {
        let do_capture =
            self.capture_backtrace_for_reports_with_children || !report.children().is_empty();
        if do_capture && let Some(backtrace) = Backtrace::capture(&self.filter) {
            let attachment = if self.show_full_path {
                ReportAttachment::new_custom::<BacktraceHandler<true>>(backtrace)
            } else {
                ReportAttachment::new_custom::<BacktraceHandler<false>>(backtrace)
            };
            report.attachments_mut().push(attachment.into_dyn_any());
        }
    }
}

const fn get_rootcause_matcher() -> Option<(&'static str, usize)> {
    let location = Location::caller();
    let file = location.file();

    let Some(prefix_len) = file
        .len()
        .checked_sub("/src/hooks/builtin_hooks/backtrace.rs".len())
    else {
        return None;
    };

    let (prefix, suffix) = file.split_at(prefix_len);
    if std::path::MAIN_SEPARATOR == '/' {
        assert!(suffix.eq_ignore_ascii_case("/src/hooks/builtin_hooks/backtrace.rs"));
    } else {
        assert!(suffix.eq_ignore_ascii_case(r#"/src\hooks\builtin_hooks\backtrace.rs"#));
    }

    let (matcher_prefix, _) = file.split_at(prefix_len + 4);

    let mut splitter_prefix = prefix;
    while !splitter_prefix.is_empty() {
        let (new_prfix, last_char) = splitter_prefix.split_at(splitter_prefix.len() - 1);
        splitter_prefix = new_prfix;
        if last_char.eq_ignore_ascii_case(std::path::MAIN_SEPARATOR_STR) {
            break;
        }
    }

    Some((matcher_prefix, splitter_prefix.len()))
}

const ROOTCAUSE_MATCHER: Option<(&str, usize)> = get_rootcause_matcher();

impl Backtrace {
    /// Captures the current stack backtrace, applying optional filtering.
    pub fn capture(filter: &BacktraceFilter) -> Option<Self> {
        let mut initial_filtering = !filter.skipped_initial_crates.is_empty();
        let mut entries: Vec<BacktraceEntry> = Vec::new();
        let mut total_omitted_frames = 0;

        let mut currently_omitted_crate_name: Option<&'static str> = None;
        let mut currently_omitted_frames = 0;

        backtrace::trace(|frame| {
            backtrace::resolve_frame(frame, |symbol| {
                // Don't consider frames without symbol names or filenames.
                let (Some(sym), Some(filename_raw)) = (symbol.name(), symbol.filename_raw()) else {
                    return;
                };

                if entries.len() >= filter.max_entry_count {
                    total_omitted_frames += 1;
                    return;
                }

                let frame_path = FramePath::new(filename_raw);

                if initial_filtering {
                    if let Some(cur_crate_name) = &frame_path.crate_name
                        && filter.skipped_initial_crates.contains(&&**cur_crate_name)
                    {
                        total_omitted_frames += 1;
                        return;
                    } else {
                        initial_filtering = false;
                    }
                }

                if let Some(cur_crate_name) = &frame_path.crate_name
                    && let Some(currently_omitted_crate_name) = &currently_omitted_crate_name
                    && cur_crate_name == currently_omitted_crate_name
                {
                    currently_omitted_frames += 1;
                    total_omitted_frames += 1;
                    return;
                }

                if let Some(currently_omitted_crate_name) = currently_omitted_crate_name.take() {
                    entries.push(BacktraceEntry::OmittedFrames {
                        count: currently_omitted_frames,
                        skipped_crate: currently_omitted_crate_name,
                    });
                    currently_omitted_frames = 0;
                }

                if let Some(cur_crate_name) = &frame_path.crate_name
                    && let Some(skipped_crate) = filter
                        .skipped_middle_crates
                        .iter()
                        .find(|&crate_name| crate_name == cur_crate_name)
                {
                    currently_omitted_crate_name = Some(skipped_crate);
                    currently_omitted_frames = 1;
                    total_omitted_frames += 1;
                    return;
                }

                entries.push(BacktraceEntry::Frame(Frame {
                    sym_demangled: format!("{sym:#}"),
                    frame_path: Some(frame_path),
                    lineno: symbol.lineno(),
                }));
            });

            true
        });

        if let Some(currently_omitted_crate_name) = currently_omitted_crate_name.take() {
            entries.push(BacktraceEntry::OmittedFrames {
                count: currently_omitted_frames,
                skipped_crate: currently_omitted_crate_name,
            });
        }

        while let Some(last) = entries.last() {
            match last {
                BacktraceEntry::Frame(frame) => {
                    if let Some(frame_path) = &frame.frame_path
                        && let Some(crate_name) = &frame_path.crate_name
                        && filter.skipped_final_crates.contains(&&**crate_name)
                    {
                        total_omitted_frames += 1;
                        entries.pop();
                    } else {
                        break;
                    }
                }
                BacktraceEntry::OmittedFrames {
                    skipped_crate,
                    count,
                } => {
                    if filter.skipped_final_crates.contains(skipped_crate) {
                        total_omitted_frames += count;
                        entries.pop();
                    } else {
                        break;
                    }
                }
            }
        }

        if entries.is_empty() && total_omitted_frames == 0 {
            None
        } else {
            Some(Self {
                entries,
                total_omitted_frames,
            })
        }
    }
}

impl FramePath {
    fn new(path: BytesOrWideString<'_>) -> Self {
        static REGEXES: OnceLock<[regex::Regex; 2]> = OnceLock::new();
        let [std_regex, registry_regex] = REGEXES.get_or_init(|| {
            [regex::Regex::new(
                r"(?:/lib/rustlib/src/rust|^/rustc/[0-9a-f]{40})/library/(std|core|alloc)/src/.*$",
            )
            .unwrap(),
            regex::Regex::new(
                r"/\.cargo/registry/src/[^/]+-[0-9a-f]{16}/([^./]+)-[0-9]+\.[^/]*/src/.*$",
            )
            .unwrap(),
            ]
        });

        let path_str = path.to_string();

        if let Some(captures) = std_regex.captures(&path_str) {
            let raw_path = path.into_path_buf();
            let crate_capture = captures.get(1).unwrap();
            let split = crate_capture.start();
            let (prefix, suffix) = (&path_str[..split - 1], &path_str[split..]);

            Self {
                raw_path,
                split_path: Some(FramePrefix {
                    prefix_kind: "RUST_SRC",
                    prefix: prefix.to_string(),
                    suffix: suffix.to_string(),
                }),
                crate_name: Some(crate_capture.as_str().to_string().into()),
            }
        } else if let Some(captures) = registry_regex.captures(&path_str) {
            let raw_path = path.into_path_buf();
            let crate_capture = captures.get(1).unwrap();
            let split = crate_capture.start();
            let (prefix, suffix) = (&path_str[..split - 1], &path_str[split..]);

            Self {
                raw_path,
                crate_name: Some(crate_capture.as_str().to_string().into()),
                split_path: Some(FramePrefix {
                    prefix_kind: "CARGO",
                    prefix: prefix.to_string(),
                    suffix: suffix.to_string(),
                }),
            }
        } else if let Some((rootcause_matcher_prefix, rootcause_splitter_prefix_len)) =
            ROOTCAUSE_MATCHER
            && path_str.starts_with(rootcause_matcher_prefix)
        {
            let raw_path = path.into_path_buf();
            let (prefix, suffix) = (
                &path_str[..rootcause_splitter_prefix_len],
                &path_str[rootcause_splitter_prefix_len + 1..],
            );
            Self {
                raw_path,
                split_path: Some(FramePrefix {
                    prefix_kind: "ROOTCAUSE",
                    prefix: prefix.to_string(),
                    suffix: suffix.to_string(),
                }),
                crate_name: Some(Cow::Borrowed("rootcause")),
            }
        } else {
            let raw_path = path.into_path_buf();
            Self {
                raw_path,
                crate_name: None,
                split_path: None,
            }
        }
    }
}
