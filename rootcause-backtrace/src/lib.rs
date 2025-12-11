#![deny(
    missing_docs,
    unsafe_code,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::broken_intra_doc_links,
    missing_copy_implementations,
    unused_doc_comments
)]

//! Stack backtrace attachment collector for rootcause error reports.
//!
//! This crate provides functionality to automatically capture and attach stack
//! backtraces to error reports. This is useful for debugging to see the call
//! stack that led to an error.
//!
//! # Quick Start
//!
//! ## Using Hooks (Automatic for All Errors)
//!
//! Register a backtrace collector as a hook to automatically capture backtraces
//! for all errors:
//!
//! ```rust
//! use rootcause::hooks::Hooks;
//! use rootcause_backtrace::BacktraceCollector;
//!
//! // Capture backtraces for all errors
//! Hooks::new()
//!     .report_creation_hook(BacktraceCollector::new_from_env())
//!     .install()
//!     .expect("failed to install hooks");
//!
//! // Now all errors automatically get backtraces!
//! fn example() -> rootcause::Report {
//!     rootcause::report!("something went wrong")
//! }
//! println!("{}", example().context("additional context"));
//! ```
//!
//! This will print a backtrace similar to the following:
//! ```text
//!  ● additional context
//!  ├ src/main.rs:12
//!  ├ Backtrace
//!  │ │ main - /build/src/main.rs:12
//!  │ │ note: 39 frame(s) omitted. For a complete backtrace, set RUST_BACKTRACE=full.
//!  │ ╰─
//!  │
//!  ● something went wrong
//!  ├ src/main.rs:10
//!  ╰ Backtrace
//!    │ example - /build/src/main.rs:10
//!    │ main    - /build/src/main.rs:12
//!    │ note: 40 frame(s) omitted. For a complete backtrace, set RUST_BACKTRACE=full.
//!    ╰─
//! ```
//!
//! ## Manual Attachment (Per-Error)
//!
//! Attach backtraces to specific errors using the extension trait:
//!
//! ```rust
//! use std::io;
//!
//! use rootcause::{Report, report};
//! use rootcause_backtrace::BacktraceExt;
//!
//! fn operation() -> Result<(), Report> {
//!     Err(report!("operation failed"))
//! }
//!
//! // Attach backtrace to the error in the Result
//! let result = operation().attach_backtrace();
//! ```
//!
//! # Environment Variables
//!
//! - `RUST_BACKTRACE=full` - Disables filtering and shows full paths
//! - `ROOTCAUSE_BACKTRACE` - Comma-separated options:
//!   - `leafs` - Only capture backtraces for leaf errors (errors without
//!     children)
//!   - `full_paths` - Show full file paths in backtraces
//!
//! # Path privacy
//!
//! By default, backtrace paths are shortened paths for improved readability,
//! but this may still expose private file system structure when a path is not
//! recognized as belonging to a known prefix (e.g., RUST_SRC).
//!
//! If exposing private file system paths is a concern, then we recommend using
//! the `--remap-path-prefix` option of `rustc` to remap source paths to
//! generic placeholders.
//!
//! A good default way to handle this is to set the following environment
//! variables when building your application for release:
//!
//! ```sh
//! export RUSTFLAGS="--remap-path-prefix=$HOME=/home/user --remap-path-prefix=$PWD=/build"
//! ```
//!
//! # Debugging symbols in release builds
//!
//! To ensure that backtraces contain useful symbol and source location
//! information in release builds, make sure to enable debug symbols in your
//! `Cargo.toml`:
//!
//! ```toml
//! [profile.release]
//! strip = false
//! # You can also set this to "line-tables-only" for smaller binaries
//! debug = true
//! ```
//!
//! # Filtering
//!
//! Control which frames appear in backtraces:
//!
//! ```rust
//! use rootcause_backtrace::{BacktraceCollector, BacktraceFilter};
//!
//! let collector = BacktraceCollector {
//!     filter: BacktraceFilter {
//!         skipped_initial_crates: &["rootcause", "rootcause-backtrace"],  // Skip frames from rootcause at start
//!         skipped_middle_crates: &["tokio"],     // Skip tokio frames in middle
//!         skipped_final_crates: &["std"],        // Skip std frames at end
//!         max_entry_count: 15,                   // Limit to 15 frames
//!         show_full_path: false,                 // Show shortened paths
//!     },
//!     capture_backtrace_for_reports_with_children: false,  // Only leaf errors
//! };
//! ```

use std::{borrow::Cow, fmt, panic::Location, sync::OnceLock};

use backtrace::BytesOrWideString;
use rootcause::{
    Report, ReportMut,
    handlers::{
        AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
        FormattingFunction,
    },
    hooks::report_creation::ReportCreationHook,
    markers::{self, Dynamic, ObjectMarkerFor},
    report_attachment::ReportAttachment,
};

/// Stack backtrace information.
///
/// Contains a collection of stack frames representing the call stack
/// at the point where a report was created.
///
/// # Examples
///
/// Capture a backtrace manually:
///
/// ```rust
/// use rootcause_backtrace::{Backtrace, BacktraceFilter};
///
/// let backtrace = Backtrace::capture(&BacktraceFilter::DEFAULT);
/// if let Some(bt) = backtrace {
///     println!("Captured {} frames", bt.entries.len());
/// }
/// ```
#[derive(Debug)]
pub struct Backtrace {
    /// The entries in the backtrace, ordered from most recent to oldest.
    pub entries: Vec<BacktraceEntry>,
    /// Total number of frames that were omitted due to filtering.
    pub total_omitted_frames: usize,
}

/// A single entry in a stack backtrace.
#[derive(Debug)]
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
#[derive(Debug)]
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
    pub raw_path: String,
    /// The crate name if detected from the path.
    pub crate_name: Option<Cow<'static, str>>,
    /// Common path prefix information for shortening display.
    pub split_path: Option<FramePrefix>,
}

/// A common prefix for a frame path.
///
/// This struct represents a decomposed file path where a known prefix
/// has been identified and separated from the rest of the path.
#[derive(Debug)]
pub struct FramePrefix {
    /// The kind of prefix used to identify this prefix.
    ///
    /// Examples: `"RUST_SRC"` for Rust standard library paths,
    /// `"CARGO"` for Cargo registry crate paths,
    /// `"ROOTCAUSE"` for rootcause library paths.
    pub prefix_kind: &'static str,
    /// The full prefix path that was removed from the original path.
    ///
    /// Example: `"/home/user/.cargo/registry/src/index.crates.
    /// io-1949cf8c6b5b557f"`
    pub prefix: String,
    /// The remaining path after the prefix was removed.
    ///
    /// Example: `"indexmap-2.12.1/src/map/core/entry.rs"`
    pub suffix: String,
}

/// Handler for formatting [`Backtrace`] attachments.
#[derive(Copy, Clone)]
pub struct BacktraceHandler<const SHOW_FULL_PATH: bool>;

fn get_function_name(s: &str) -> &str {
    let mut word_start = 0usize;
    let mut word_end = 0usize;
    let mut angle_nesting_level = 0u64;
    let mut curly_nesting_level = 0u64;
    let mut potential_function_arrow = false;
    let mut inside_word = false;

    for (i, c) in s.char_indices() {
        if curly_nesting_level == 0 && angle_nesting_level == 0 {
            if !inside_word && unicode_ident::is_xid_start(c) {
                word_start = i;
                inside_word = true;
            } else if inside_word && !unicode_ident::is_xid_continue(c) {
                word_end = i;
                inside_word = false;
            }
        }

        let was_potential_function_arrow = potential_function_arrow;
        potential_function_arrow = c == '-';

        if c == '<' {
            angle_nesting_level = angle_nesting_level.saturating_add(1);
        } else if c == '>' && !was_potential_function_arrow {
            angle_nesting_level = angle_nesting_level.saturating_sub(1);
        } else if c == '{' {
            curly_nesting_level = curly_nesting_level.saturating_add(1);
            if !inside_word && curly_nesting_level == 1 && angle_nesting_level == 0 {
                word_start = i;
                inside_word = true;
            }
        } else if c == '}' {
            curly_nesting_level = curly_nesting_level.saturating_sub(1);
            if inside_word && curly_nesting_level == 0 {
                word_end = i + 1;
                inside_word = false;
            }
        }
    }

    if word_start < word_end {
        &s[word_start..word_end]
    } else {
        // We started at word start but never found an end; return rest of string
        &s[word_start..]
    }
}

impl<const SHOW_FULL_PATH: bool> AttachmentHandler<Backtrace> for BacktraceHandler<SHOW_FULL_PATH> {
    fn display(value: &Backtrace, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const MAX_UNWRAPPED_SYM_LENGTH: usize = 25;
        let mut max_seen_length = 0;
        for entry in &value.entries {
            if let BacktraceEntry::Frame(frame) = entry {
                let sym = get_function_name(&frame.sym_demangled);
                if sym.len() <= MAX_UNWRAPPED_SYM_LENGTH && sym.len() > max_seen_length {
                    max_seen_length = sym.len();
                }
            }
        }

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
                    let sym = get_function_name(&frame.sym_demangled);

                    if sym.len() <= MAX_UNWRAPPED_SYM_LENGTH {
                        write!(f, "{:<max_seen_length$} - ", sym)?;
                    } else {
                        write!(f, "{sym}\n   - ")?;
                    }

                    if let Some(path) = &frame.frame_path {
                        if SHOW_FULL_PATH {
                            write!(f, "{}", path.raw_path)?;
                        } else if let Some(split_path) = &path.split_path {
                            write!(f, "[..]/{}", split_path.suffix)?;
                        } else {
                            write!(f, "{}", path.raw_path)?;
                        }

                        if let Some(lineno) = frame.lineno {
                            write!(f, ":{lineno}")?;
                        }
                    }
                    writeln!(f)?;
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
        std::fmt::Debug::fmt(value, formatter)
    }

    fn preferred_formatting_style(
        backtrace: &Backtrace,
        _report_formatting_function: FormattingFunction,
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
///
/// # Examples
///
/// Basic usage with default settings:
///
/// ```rust
/// use rootcause::hooks::Hooks;
/// use rootcause_backtrace::BacktraceCollector;
///
/// Hooks::new()
///     .report_creation_hook(BacktraceCollector::new_from_env())
///     .install()
///     .expect("failed to install hooks");
/// ```
///
/// Custom configuration:
///
/// ```rust
/// use rootcause::hooks::Hooks;
/// use rootcause_backtrace::{BacktraceCollector, BacktraceFilter};
///
/// let collector = BacktraceCollector {
///     filter: BacktraceFilter {
///         skipped_initial_crates: &[],
///         skipped_middle_crates: &[],
///         skipped_final_crates: &[],
///         max_entry_count: 30,
///         show_full_path: true,
///     },
///     capture_backtrace_for_reports_with_children: true,
/// };
///
/// Hooks::new()
///     .report_creation_hook(collector)
///     .install()
///     .expect("failed to install hooks");
/// ```
#[derive(Copy, Clone)]
pub struct BacktraceCollector {
    /// Configuration for filtering and formatting backtrace frames.
    pub filter: BacktraceFilter,

    /// If set to true, a backtrace is captured for every report creation,
    /// including reports that have child reports (i.e., reports created with
    /// existing children). If set to false, a backtrace is captured only
    /// for reports created without any children. Reports created without
    /// children always receive a backtrace regardless of this setting.
    pub capture_backtrace_for_reports_with_children: bool,
}

/// Configuration for filtering frames from certain crates in a backtrace.
///
/// # Examples
///
/// Use default filtering:
///
/// ```rust
/// use rootcause_backtrace::BacktraceFilter;
///
/// let filter = BacktraceFilter::DEFAULT;
/// ```
///
/// Custom filtering to focus on application code:
///
/// ```rust
/// use rootcause_backtrace::BacktraceFilter;
///
/// let filter = BacktraceFilter {
///     // Hide rootcause crate frames at the start
///     skipped_initial_crates: &["rootcause", "rootcause-backtrace"],
///     // Hide framework frames in the middle
///     skipped_middle_crates: &["tokio", "hyper", "tower"],
///     // Hide runtime frames at the end
///     skipped_final_crates: &["std", "tokio"],
///     // Show only the most relevant 10 frames
///     max_entry_count: 10,
///     // Show shortened paths
///     show_full_path: false,
/// };
/// ```
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
    /// Whether to show full file paths in the backtrace frames.
    pub show_full_path: bool,
}

impl BacktraceFilter {
    /// Default backtrace filter settings.
    pub const DEFAULT: Self = Self {
        skipped_initial_crates: &[
            "backtrace",
            "rootcause",
            "rootcause-backtrace",
            "core",
            "std",
            "alloc",
        ],
        skipped_middle_crates: &["std", "core", "alloc", "tokio"],
        skipped_final_crates: &["std", "core", "alloc", "tokio"],
        max_entry_count: 20,
        show_full_path: false,
    };
}

impl Default for BacktraceFilter {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug)]
struct RootcauseEnvOptions {
    rust_backtrace_full: bool,
    backtrace_leafs_only: bool,
    show_full_path: bool,
}

impl RootcauseEnvOptions {
    fn get() -> &'static Self {
        static ROOTCAUSE_FLAGS: OnceLock<RootcauseEnvOptions> = OnceLock::new();

        ROOTCAUSE_FLAGS.get_or_init(|| {
            let rust_backtrace_full =
                std::env::var_os("RUST_BACKTRACE").is_some_and(|var| var == "full");
            let mut show_full_path = rust_backtrace_full;
            let mut backtrace_leafs_only = false;
            if let Some(var) = std::env::var_os("ROOTCAUSE_BACKTRACE") {
                for v in var.to_string_lossy().split(',') {
                    if v.eq_ignore_ascii_case("leafs") {
                        backtrace_leafs_only = true;
                    } else if v.eq_ignore_ascii_case("full_paths") {
                        show_full_path = true;
                    }
                }
            }
            RootcauseEnvOptions {
                rust_backtrace_full,
                backtrace_leafs_only,
                show_full_path,
            }
        })
    }
}

impl BacktraceCollector {
    /// Creates a new [`BacktraceCollector`] with default settings.
    ///
    /// Configuration is controlled by environment variables. By default,
    /// filtering is applied and backtraces are only captured for reports
    /// without children.
    ///
    /// # Environment Variables
    ///
    /// - `RUST_BACKTRACE=full` - Disables all filtering and shows all frames
    /// - `ROOTCAUSE_BACKTRACE` - Comma-separated options:
    ///   - `leafs` - Only capture backtraces for leaf errors (errors without
    ///     children)
    ///   - `full_paths` - Show full file paths instead of shortened paths
    ///
    /// The `RUST_BACKTRACE=full` setting implies `full_paths` unless explicitly
    /// overridden by `ROOTCAUSE_BACKTRACE`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rootcause::hooks::Hooks;
    /// use rootcause_backtrace::BacktraceCollector;
    ///
    /// // Respects RUST_BACKTRACE and ROOTCAUSE_BACKTRACE environment variables
    /// Hooks::new()
    ///     .report_creation_hook(BacktraceCollector::new_from_env())
    ///     .install()
    ///     .expect("failed to install hooks");
    /// ```
    pub fn new_from_env() -> Self {
        let env_options = RootcauseEnvOptions::get();
        let capture_backtrace_for_reports_with_children = !env_options.backtrace_leafs_only;

        Self {
            filter: if env_options.rust_backtrace_full {
                BacktraceFilter {
                    skipped_initial_crates: &[],
                    skipped_middle_crates: &[],
                    skipped_final_crates: &[],
                    max_entry_count: usize::MAX,
                    show_full_path: env_options.show_full_path,
                }
            } else {
                BacktraceFilter {
                    show_full_path: env_options.show_full_path,
                    ..BacktraceFilter::DEFAULT
                }
            },
            capture_backtrace_for_reports_with_children,
        }
    }
}

impl ReportCreationHook for BacktraceCollector {
    fn on_local_creation(&self, mut report: ReportMut<'_, Dynamic, markers::Local>) {
        let do_capture =
            self.capture_backtrace_for_reports_with_children || report.children().is_empty();
        if do_capture && let Some(backtrace) = Backtrace::capture(&self.filter) {
            let attachment = if self.filter.show_full_path {
                ReportAttachment::new_custom::<BacktraceHandler<true>>(backtrace)
            } else {
                ReportAttachment::new_custom::<BacktraceHandler<false>>(backtrace)
            };
            report.attachments_mut().push(attachment.into_dynamic());
        }
    }

    fn on_sendsync_creation(&self, mut report: ReportMut<'_, Dynamic, markers::SendSync>) {
        let do_capture =
            self.capture_backtrace_for_reports_with_children || report.children().is_empty();
        if do_capture && let Some(backtrace) = Backtrace::capture(&self.filter) {
            let attachment = if self.filter.show_full_path {
                ReportAttachment::new_custom::<BacktraceHandler<true>>(backtrace)
            } else {
                ReportAttachment::new_custom::<BacktraceHandler<false>>(backtrace)
            };
            report.attachments_mut().push(attachment.into_dynamic());
        }
    }
}

const fn get_rootcause_backtrace_matcher(
    location: &'static Location<'static>,
) -> Option<(&'static str, usize)> {
    let file = location.file();

    let Some(prefix_len) = file.len().checked_sub("/src/lib.rs".len()) else {
        return None;
    };

    let (prefix, suffix) = file.split_at(prefix_len);
    // Assert the suffix is /src/lib.rs (or \src\lib.rs on Windows)
    // This is a compile-time check that the caller location is valid
    if std::path::MAIN_SEPARATOR == '/' {
        assert!(suffix.eq_ignore_ascii_case("/src/lib.rs"));
    } else {
        assert!(suffix.eq_ignore_ascii_case(r#"/src\lib.rs"#));
    }

    let (matcher_prefix, _) = file.split_at(prefix_len + 4);

    let mut splitter_prefix = prefix;
    while !splitter_prefix.is_empty() {
        let (new_prefix, last_char) = splitter_prefix.split_at(splitter_prefix.len() - 1);
        splitter_prefix = new_prefix;
        if last_char.eq_ignore_ascii_case(std::path::MAIN_SEPARATOR_STR) {
            break;
        }
    }

    Some((matcher_prefix, splitter_prefix.len()))
}

const ROOTCAUSE_BACKTRACE_MATCHER: Option<(&str, usize)> =
    get_rootcause_backtrace_matcher(Location::caller());
const ROOTCAUSE_MATCHER: Option<(&str, usize)> =
    get_rootcause_backtrace_matcher(rootcause::__private::ROOTCAUSE_LOCATION);

impl Backtrace {
    /// Captures the current stack backtrace, applying optional filtering.
    pub fn capture(filter: &BacktraceFilter) -> Option<Self> {
        let mut initial_filtering = !filter.skipped_initial_crates.is_empty();
        let mut entries: Vec<BacktraceEntry> = Vec::new();
        let mut total_omitted_frames = 0;

        let mut delayed_omitted_frame: Option<Frame> = None;
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
                    delayed_omitted_frame = None;
                    currently_omitted_frames += 1;
                    total_omitted_frames += 1;
                    return;
                }

                if let Some(currently_omitted_crate_name) = currently_omitted_crate_name.take() {
                    if let Some(delayed_frame) = delayed_omitted_frame.take() {
                        entries.push(BacktraceEntry::Frame(delayed_frame));
                    } else {
                        entries.push(BacktraceEntry::OmittedFrames {
                            count: currently_omitted_frames,
                            skipped_crate: currently_omitted_crate_name,
                        });
                    }
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
                    delayed_omitted_frame = Some(Frame {
                        sym_demangled: format!("{sym:#}"),
                        frame_path: Some(frame_path),
                        lineno: symbol.lineno(),
                    });
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
            if let Some(delayed_frame) = delayed_omitted_frame.take() {
                entries.push(BacktraceEntry::Frame(delayed_frame));
            } else {
                entries.push(BacktraceEntry::OmittedFrames {
                    count: currently_omitted_frames,
                    skipped_crate: currently_omitted_crate_name,
                });
            }
        }

        while let Some(last) = entries.last() {
            match last {
                BacktraceEntry::Frame(frame) => {
                    let mut skip = false;
                    if let Some(frame_path) = &frame.frame_path
                        && let Some(crate_name) = &frame_path.crate_name
                        && filter.skipped_final_crates.contains(&&**crate_name)
                    {
                        skip = true;
                    } else if frame.sym_demangled == "__libc_start_call_main"
                        || frame.sym_demangled == "__libc_start_main_impl"
                    {
                        skip = true;
                    } else if let Some(frame_path) = &frame.frame_path
                        && frame.sym_demangled == "_start"
                        && frame_path.raw_path.contains("zig/libc/glibc")
                    {
                        skip = true;
                    }

                    if skip {
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
            [
                // Matches Rust standard library paths:
                // - /lib/rustlib/src/rust/library/{std|core|alloc}/src/...
                // - /rustc/{40-char-hash}/library/{std|core|alloc}/src/...
                regex::Regex::new(
                    r"(?:/lib/rustlib/src/rust|^/rustc/[0-9a-f]{40})/library/(std|core|alloc)/src/.*$",
                )
                .expect("built-in regex pattern for std library paths should be valid"),
                // Matches Cargo registry paths:
                // - /.cargo/registry/src/{index}-{16-char-hash}/{crate}-{version}/src/...
                regex::Regex::new(
                    r"/\.cargo/registry/src/[^/]+-[0-9a-f]{16}/([^./]+)-[0-9]+\.[^/]*/src/.*$",
                )
                .expect("built-in regex pattern for cargo registry paths should be valid"),
            ]
        });

        let path_str = path.to_string();

        if let Some(captures) = std_regex.captures(&path_str) {
            let raw_path = path.to_str_lossy().into_owned();
            let crate_capture = captures
                .get(1)
                .expect("regex capture group 1 should exist for std library paths");
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
            let raw_path = path.to_str_lossy().into_owned();
            let crate_capture = captures
                .get(1)
                .expect("regex capture group 1 should exist for cargo registry paths");
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
            let raw_path = path.to_str_lossy().into_owned();
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
        } else if let Some((rootcause_matcher_prefix, rootcause_splitter_prefix_len)) =
            ROOTCAUSE_BACKTRACE_MATCHER
            && path_str.starts_with(rootcause_matcher_prefix)
        {
            let raw_path = path.to_str_lossy().into_owned();
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
                crate_name: Some(Cow::Borrowed("rootcause-backtrace")),
            }
        } else {
            let raw_path = path.to_str_lossy().into_owned();
            Self {
                raw_path,
                crate_name: None,
                split_path: None,
            }
        }
    }
}

/// Extension trait for attaching backtraces to reports.
///
/// This trait provides methods to easily attach a captured backtrace to a
/// report or to the error contained within a `Result`.
///
/// # Examples
///
/// Attach backtrace to a report:
///
/// ```rust
/// use std::io;
///
/// use rootcause::report;
/// use rootcause_backtrace::BacktraceExt;
///
/// let report = report!(io::Error::other("An error occurred")).attach_backtrace();
/// ```
///
/// Attach backtrace to a `Result`:
///
/// ```rust
/// use std::io;
///
/// use rootcause::{Report, report};
/// use rootcause_backtrace::BacktraceExt;
///
/// fn might_fail() -> Result<(), Report> {
///     Err(report!(io::Error::other("operation failed")).into_dynamic())
/// }
///
/// let result = might_fail().attach_backtrace();
/// ```
///
/// Use a custom filter:
///
/// ```rust
/// use std::io;
///
/// use rootcause::report;
/// use rootcause_backtrace::{BacktraceExt, BacktraceFilter};
///
/// let filter = BacktraceFilter {
///     skipped_initial_crates: &[],
///     skipped_middle_crates: &[],
///     skipped_final_crates: &[],
///     max_entry_count: 50,
///     show_full_path: true,
/// };
///
/// let report = report!(io::Error::other("detailed error")).attach_backtrace_with_filter(&filter);
/// ```
pub trait BacktraceExt: Sized {
    /// Attaches a captured backtrace to the report using the default filter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io;
    ///
    /// use rootcause::report;
    /// use rootcause_backtrace::BacktraceExt;
    ///
    /// let report = report!(io::Error::other("error")).attach_backtrace();
    /// ```
    fn attach_backtrace(self) -> Self {
        self.attach_backtrace_with_filter(&BacktraceFilter::DEFAULT)
    }

    /// Attaches a captured backtrace to the report using the specified filter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io;
    ///
    /// use rootcause::report;
    /// use rootcause_backtrace::{BacktraceExt, BacktraceFilter};
    ///
    /// let filter = BacktraceFilter {
    ///     max_entry_count: 10,
    ///     ..BacktraceFilter::DEFAULT
    /// };
    ///
    /// let report = report!(io::Error::other("error")).attach_backtrace_with_filter(&filter);
    /// ```
    fn attach_backtrace_with_filter(self, filter: &BacktraceFilter) -> Self;
}

impl<C: ?Sized, T> BacktraceExt for Report<C, markers::Mutable, T>
where
    Backtrace: ObjectMarkerFor<T>,
{
    fn attach_backtrace_with_filter(mut self, filter: &BacktraceFilter) -> Self {
        if let Some(backtrace) = Backtrace::capture(&filter) {
            if filter.show_full_path {
                self = self.attach_custom::<BacktraceHandler<true>, _>(backtrace);
            } else {
                self = self.attach_custom::<BacktraceHandler<false>, _>(backtrace);
            }
        }
        self
    }
}

impl<C: ?Sized, V, T> BacktraceExt for Result<V, Report<C, markers::Mutable, T>>
where
    Backtrace: ObjectMarkerFor<T>,
{
    fn attach_backtrace_with_filter(self, filter: &BacktraceFilter) -> Self {
        match self {
            Ok(v) => Ok(v),
            Err(report) => Err(report.attach_backtrace_with_filter(filter)),
        }
    }
}
