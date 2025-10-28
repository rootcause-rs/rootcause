//! Stack backtrace attachment collector.
//!
//! This module provides functionality to automatically capture and attach stack
//! backtraces to reports when they are created. This is useful for debugging to
//! see the call stack that led to an error.
//!
//! ## Feature Requirement
//!
//! This module is only available when the `backtrace` feature is enabled in
//! `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rootcause = { version = "0.3", features = ["backtrace"] }
//! ```
//!
//! ## Usage
//!
//! The [`BacktraceCollector`] can be registered as an attachment collector hook
//! to automatically capture backtraces for all reports:
//!
//! ```rust
//! use rootcause::hooks::{
//!     attachment_collectors::backtrace::BacktraceCollector, register_attachment_collector_hook,
//! };
//!
//! register_attachment_collector_hook(BacktraceCollector::default());
//! ```
//!
//! Once registered, all reports will automatically include a backtrace showing
//! the call stack from where the report was created.

use core::fmt;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use backtrace::BytesOrWideString;
use indexmap::{IndexMap, IndexSet};
use rootcause_internals::handlers::{
    AttachmentFormattingPlacement, AttachmentFormattingStyle, AttachmentHandler,
};

use crate::hooks::AttachmentCollectorHook;

/// Stack backtrace information.
///
/// Contains a collection of stack frames representing the call stack
/// at the point where a report was created.
pub struct Backtrace {
    /// The stack frames in the backtrace, ordered from most recent to oldest.
    pub frames: Vec<Frame>,
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
    /// Whether this frame was detected as part of tokio or std library code.
    pub detected_as_tokio_or_std: bool,
}

/// File path information for a stack frame.
///
/// Contains the raw path and processed components for better display
/// formatting.
pub struct FramePath {
    /// The raw file path from the debug information.
    pub raw_path: PathBuf,
    /// Common path prefix information for shortening display.
    pub prefix: Option<FramePrefix>,
    /// The remaining path suffix after removing the prefix.
    pub suffix: String,
}

pub struct FramePrefix {
    pub key: String,
    pub value: String,
}

#[derive(Copy, Clone)]
pub struct BacktraceHandler;

impl AttachmentHandler<Backtrace> for BacktraceHandler {
    fn display(value: &Backtrace, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut seen_prefixes = IndexMap::<&str, IndexSet<&str>>::new();
        for frame in &value.frames {
            writeln!(f, "{}", frame.sym_demangled)?;

            if let Some(path) = &frame.frame_path {
                write!(f, "  @ ")?;
                if let Some(prefix) = &path.prefix {
                    let (index, _) = seen_prefixes
                        .entry(&prefix.key)
                        .or_default()
                        .insert_full(&prefix.value);
                    if index == 0 {
                        write!(f, "[{}]/", prefix.key)?;
                    } else {
                        write!(f, "[{}-{index}]/", prefix.value)?;
                    }
                }
                write!(f, "{}", path.suffix)?;
                if let Some(lineno) = frame.lineno {
                    writeln!(f, ":{lineno}")?;
                } else {
                    writeln!(f)?;
                }
            }
        }
        if !seen_prefixes.is_empty() {
            writeln!(f)?;
            for (prefix_key, paths) in seen_prefixes {
                for (index, prefix) in paths.iter().enumerate() {
                    if index == 0 {
                        writeln!(f, "[{}]: {prefix}", prefix_key)?;
                    } else {
                        writeln!(f, "[{}-{index}]: {prefix}", prefix_key)?;
                    }
                }
            }
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
            placement: if backtrace.frames.is_empty() {
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

#[derive(Copy, Clone)]
pub struct BacktraceCollector {
    pub max_frame_count: usize,
    pub do_capture_filtering: bool,
}

impl Default for BacktraceCollector {
    fn default() -> Self {
        Self {
            do_capture_filtering: std::env::var_os("RUST_BACKTRACE")
                .is_none_or(|v| !v.eq_ignore_ascii_case("full")),
            max_frame_count: 100,
        }
    }
}

impl AttachmentCollectorHook<Backtrace> for BacktraceCollector {
    type Handler = BacktraceHandler;

    fn collect(&self) -> Backtrace {
        Backtrace {
            frames: capture_backtrace(self.max_frame_count, self.do_capture_filtering),
        }
    }
}

// Based on https://github.com/rust-lang/rust/blob/1f7dcc878d73c45cc40018aac6e5c767446df110/library/std/src/sys/backtrace.rs#L46
fn capture_backtrace(max_frame_count: usize, do_capture_filtering: bool) -> Vec<Frame> {
    let cwd = std::env::current_dir().ok();
    let cwd: Option<&Path> = cwd.as_deref();

    #[derive(PartialEq, Eq, Debug)]
    enum CaptureState {
        LookingForCreationHook,
        LookingForDiffentCrate,
        Capturing,
        Done,
    }
    let mut state = if do_capture_filtering {
        CaptureState::LookingForCreationHook
    } else {
        CaptureState::Capturing
    };
    let mut frames: Vec<Frame> = Vec::new();

    macro_rules! is_done {
        () => {
            frames.len() >= max_frame_count || state == CaptureState::Done
        };
    }

    backtrace::trace(|frame| {
        if is_done!() {
            return false;
        }

        backtrace::resolve_frame(frame, |symbol| {
            if is_done!() {
                return;
            }

            let mut calculated_frame_path = None;
            // Borrow checker rules...
            macro_rules! get_frame_path {
                () => {
                    calculated_frame_path.get_or_insert_with(|| {
                        if let Some(filename_raw) = symbol.filename_raw() {
                            let (frame_path, path_is_tokio_or_std, path_is_rootcause) =
                                FramePath::new(cwd, filename_raw);
                            (Some(frame_path), path_is_tokio_or_std, path_is_rootcause)
                        } else {
                            (None, false, false)
                        }
                    })
                };
            }

            if let Some(sym) = symbol.name() {
                let sym_demangled = format!("{sym:#}");
                match state {
                    CaptureState::LookingForCreationHook => {
                        if sym_demangled.contains("__run_creation_hooks") {
                            state = CaptureState::LookingForDiffentCrate;
                        }
                        return;
                    }
                    CaptureState::LookingForDiffentCrate => {
                        let is_rootcause = sym_demangled.starts_with("rootcause::")
                            || sym_demangled.contains(" as rootcause::")
                            || get_frame_path!().2;
                        if !is_rootcause {
                            state = CaptureState::Capturing;
                        } else {
                            return;
                        }
                    }
                    CaptureState::Capturing => {
                        if do_capture_filtering
                            && [
                                "std::sys::backtrace__rust_begin_short_backtrace",
                                "backtrace__rust_begin_short_backtrace",
                                "tokio::runtime::context::runtime::enter_runtime",
                            ]
                            .contains(&sym_demangled.as_str())
                        {
                            state = CaptureState::Done;
                            return;
                        }
                    }
                    CaptureState::Done => {
                        return;
                    }
                }
                if state == CaptureState::Capturing {
                    let (frame_path, path_is_tokio_or_std, _path_is_rootcause) = get_frame_path!();

                    frames.push(Frame {
                        detected_as_tokio_or_std: *path_is_tokio_or_std
                            || sym_demangled.starts_with("std::")
                            || sym_demangled.starts_with("tokio::")
                            || sym_demangled == "__rust_try"
                            || sym_demangled == "__GI___clone3"
                            || sym_demangled == "start_thread",
                        sym_demangled,
                        frame_path: frame_path.take(),
                        lineno: symbol.lineno(),
                    });
                }
            }
        });

        !(is_done!())
    });

    while let Some(last) = frames.last()
        && last.detected_as_tokio_or_std
    {
        frames.pop();
    }
    frames
}

impl FramePath {
    fn new(cwd: Option<&Path>, path: BytesOrWideString<'_>) -> (Self, bool, bool) {
        static REGEXES: OnceLock<[regex::Regex; 2]> = OnceLock::new();
        let [std_regex, registry_regex] = REGEXES.get_or_init(|| {
            [regex::Regex::new(
                r"(?:/lib/rustlib/src/rust|^/rustc/[0-9a-f]{40})/library/(std|core|alloc)/src/.*$",
            )
            .unwrap(),
            regex::Regex::new(
                r"/\.cargo/registry/src/([^/]+-[0-9a-f]{16})/([^./]+)-[0-9]+\.[^/]*/src/.*$",
            )
            .unwrap(),
            ]
        });

        let path_str = path.to_string();

        if let Some(captures) = std_regex.captures(&path_str) {
            let raw_path = path.into_path_buf();
            let crate_capture = captures.get(1).unwrap();
            let split = crate_capture.start();
            let (prefix, suffix) = path_str.split_at(split);

            (
                Self {
                    raw_path,
                    prefix: Some(FramePrefix {
                        key: String::from("rust-src"),
                        value: prefix.to_string(),
                    }),
                    suffix: suffix.to_string(),
                },
                true,
                false,
            )
        } else if let Some(captures) = registry_regex.captures(&path_str) {
            let raw_path = path.into_path_buf();
            let crate_capture = captures.get(2).unwrap();
            let is_tokio = crate_capture.as_str() == "tokio";
            let is_rootcause = crate_capture.as_str() == "rootcause";
            let split = crate_capture.start();
            let (prefix, suffix) = path_str.split_at(split);

            (
                Self {
                    raw_path,
                    prefix: Some(FramePrefix {
                        key: captures.get(1).unwrap().as_str().to_string(),
                        value: prefix.to_string(),
                    }),
                    suffix: suffix.to_string(),
                },
                is_tokio,
                is_rootcause,
            )
        } else {
            let raw_path = path.into_path_buf();
            if let Some(cwd) = cwd
                && let Ok(stripped) = raw_path.strip_prefix(cwd)
            {
                (
                    Self {
                        prefix: None,
                        suffix: stripped.to_string_lossy().into_owned(),
                        raw_path,
                    },
                    false,
                    false,
                )
            } else {
                (
                    Self {
                        raw_path,
                        prefix: None,
                        suffix: path_str.to_string(),
                    },
                    false,
                    false,
                )
            }
        }
    }
}
