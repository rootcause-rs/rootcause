# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `as_dyn_error()` on `Report`, `ReportRef`, and `ReportMut` for an explicit `&dyn Error` view; the `SendSync` variants return `dyn Error + Send + Sync` [#189](https://github.com/rootcause-rs/rootcause/pull/189).
- Added a compatibility module for error-stack v0.8 (behind the `compat-error-stack08` feature flag) [#191](https://github.com/rootcause-rs/rootcause/pull/191).
- `ReportAttachmentRef::format_inner_with_parent` for formatting an attachment with parent-report context. [#163](https://github.com/rootcause-rs/rootcause/pull/163).

### Changed

- `DefaultReportFormatter` now populates the `AttachmentParent` argument when invoking `AttachmentFormatterHook::display`/`debug` (previously always `None`), exposing the parent report and the attachment's pre-sort index. [#163](https://github.com/rootcause-rs/rootcause/pull/163).

### Removed

- The `Deref<Target = dyn Error>` impls on `Report`, `ReportRef`, and `ReportMut` (added in 0.13.0), which broke type inference and interfered with `into_report()` and `Report::from(...)`. Use `as_dyn_error()` or the `AsRef<dyn Error>` impls instead [#189](https://github.com/rootcause-rs/rootcause/pull/189).
- The compatibility modules for error-stack v0.5, v0.6, and v0.7 (`compat-error-stack05`, `compat-error-stack06`, and `compat-error-stack07` feature flags). All error-stack versions before 0.8.0 are affected by the soundness issue [RUSTSEC-2026-0198](https://rustsec.org/advisories/RUSTSEC-2026-0198); use the new `compat-error-stack08` module instead [#191](https://github.com/rootcause-rs/rootcause/pull/191).

## [0.13.0] - 2026-06-14

### Added

- `&dyn Any` accessors for contexts and attachments via `current_context_as_any(_mut)` on `Report`/`ReportRef`/`ReportMut` and `inner_as_any(_mut)` on `ReportAttachment`/`ReportAttachmentRef`/`ReportAttachmentMut` [#151](https://github.com/rootcause-rs/rootcause/issues/151).
- Added a compatibility module for error-stack v0.7 [#169](https://github.com/rootcause-rs/rootcause/pull/169).
- Implement `Deref<Target = dyn Error>` and `AsRef<dyn Error>` for `Report`, `ReportRef` and `ReportMut`; `Report<_, _, SendSync>` targets `dyn Error + Send + Sync`. May break type inference at some `Report::new*` call sites. [#147](https://github.com/rootcause-rs/rootcause/pull/147), [#149](https://github.com/rootcause-rs/rootcause/pull/149)
- Doc examples for nearly all public functions [#136](https://github.com/rootcause-rs/rootcause/pull/136).

### Changed

- Moved the preformat functionality out of `rootcause` into the new `rootcause-preformat` companion crate. The `preformatted` module, the `preformat*` methods, and the `display_preformatted`/`debug_preformatted` formatter-hook methods are removed; use the `Preformat*Ext` and `ContextTransformNestedExt` traits from the new crate. `AttachmentFormatterHook::preferred_formatting_style` and `ContextFormatterHook::preferred_context_formatting_style` now take a concrete type instead of `Dynamic`. [#148](https://github.com/rootcause-rs/rootcause/pull/148)

### Fixed

- Fixed cross-compilation support in `rootcause-backtrace` by detecting the path separator from `Location::caller()` at compile time instead of from a build-script-recorded host constant [#134](https://github.com/rootcause-rs/rootcause/pull/134).

## [0.12.1] - 2026-02-19

### Added

- Implement `IntoIterator` for `&mut ReportAttachments` [#128](https://github.com/rootcause-rs/rootcause/pull/128).
- Add a `ReportAttachment::inner_mut()` method [#128](https://github.com/rootcause-rs/rootcause/pull/128).

## [0.12.0] - 2026-02-07

### Added

- Added support for formatting the error sources for a context [#94](https://github.com/rootcause-rs/rootcause/pull/94).
  - This adds new fields to the `ContextFormattingStyle` and the `DefaultReportFormatter`.
- Added support for mutating attachments [#113](https://github.com/rootcause-rs/rootcause/pull/113).

### Changed

- The contexts and attachments will by default use the same formatter (`Display`/`Debug`) as the one used to format the report. [#116](https://github.com/rootcause-rs/rootcause/pull/116)

### Fixed

- Fixed some issues with using rootcause-backtrace on windows [#118](https://github.com/rootcause-rs/rootcause/pull/118), [#120](https://github.com/rootcause-rs/rootcause/pull/120), [#121](https://github.com/rootcause-rs/rootcause/pull/121).

## [0.11.1] - 2026-01-03

### Added

- Added a `OptionExt` trait [#92](https://github.com/rootcause-rs/rootcause/pull/92)
- Added a `rootcause::Result` type alias [#91](https://github.com/rootcause-rs/rootcause/pull/91)
- Added methods to get the type name of contexts and attachments [#100](https://github.com/rootcause-rs/rootcause/pull/100)
- Implements `Clone` for `Backtrace` and `Display` for `Location` [#100](https://github.com/rootcause-rs/rootcause/pull/100)
- Added `ReportMut::attach` and `ReportMut::attach_custom` [#101](https://github.com/rootcause-rs/rootcause/pull/101)
- Added a `rootcause-tracing` crate [#102](https://github.com/rootcause-rs/rootcause/pull/102)

### Fixed

- Added `#[track_caller]` to two functions that were missing them [#89](https://github.com/rootcause-rs/rootcause/pull/89)

## [0.11.0] - 2025-12-12

### Added

- Added a compatibility module for boxed errors [#70](https://github.com/rootcause-rs/rootcause/pull/70)
- Added a compatibility module for error-stack v0.5 [#75](https://github.com/rootcause-rs/rootcause/pull/75)
- Added a `ReportConversion` trait along with `context_to`, `context_transform` and `context_transform_nested` methods [#83](https://github.com/rootcause-rs/rootcause/pull/83)

### Changed

- The default report formatter no longer uses ANSI colors [#74](https://github.com/rootcause-rs/rootcause/pull/74)
- Re-organize the compatibility modules [#75](https://github.com/rootcause-rs/rootcause/pull/75)
- The `dyn Any`-marker for type-erased reports was replaced with a custom `Dynamic` marker [#78](https://github.com/rootcause-rs/rootcause/pull/78)
- Implement a new hook system [#80](https://github.com/rootcause-rs/rootcause/pull/80), [#81](https://github.com/rootcause-rs/rootcause/pull/81)
- Moved backtrace support into the new `rootcause-backtrace` crate [#82](https://github.com/rootcause-rs/rootcause/pull/82)

## [0.10.0] - 2025-11-24

### Fixed

- Fix issue [#64](https://github.com/rootcause-rs/rootcause/issues/64) and parts of [#63](https://github.com/rootcause-rs/rootcause/issues/63) by removing a lot of trait bounds [#67](https://github.com/rootcause-rs/rootcause/pull/67)
- Implement `Unpin` for most types. This fixes the other half of [#63](https://github.com/rootcause-rs/rootcause/issues/63). [#68](https://github.com/rootcause-rs/rootcause/pull/68)

## [0.9.1] - 2025-11-23

### Fixed

- Fixed the building of docs on docs.rs [#65](https://github.com/rootcause-rs/rootcause/pull/65)

## [0.9.0] - 2025-11-22

### Added

- Added a new `compat` module and populated it with `eyre` and `error-stack` compatibility [#55](https://github.com/rootcause-rs/rootcause/pull/55)
- Added a `format_with_hook` method on reports to format a report using a specific hook [#57](https://github.com/rootcause-rs/rootcause/pull/57)

### Changed

- Refactored the `anyhow_compat` module into the new `compat` module [#55](https://github.com/rootcause-rs/rootcause/pull/55)

### Fixed

- Removed an unintentional dependency on triomphe with default-features turned on [#61](https://github.com/rootcause-rs/rootcause/pull/61)

## [0.8.1] - 2025-11-20

### Added

- Added an `anyhow` feature which adds compatibility traits for going back and forth between anyhow [#51](https://github.com/rootcause-rs/rootcause/pull/51)

## [0.8.0] - 2025-11-19

### Added

- More safety reasoning [#47](https://github.com/rootcause-rs/rootcause/pull/47)

### Changed

- Change the formatting of Backtraces and Location and how they are customized [#44](https://github.com/rootcause-rs/rootcause/pull/44)
- `ReportMut::reborrow` has been renamed to `ReportMut::as_mut` [#47](https://github.com/rootcause-rs/rootcause/pull/47)
- Update the backtrace formatting [#48](https://github.com/rootcause-rs/rootcause/pull/48)

## [0.7.0] - 2025-11-06

### Added

- More docs, examples, README [#40](https://github.com/rootcause-rs/rootcause/pull/40), [#41](https://github.com/rootcause-rs/rootcause/pull/41)

## [0.6.0] - 2025-10-29

### Fixed

- Fix the links in the CHANGELOG [#34](https://github.com/rootcause-rs/rootcause/pull/34)

### Added

- More docs [#35](https://github.com/rootcause-rs/rootcause/pull/35), [#37](https://github.com/rootcause-rs/rootcause/pull/37), [#38](https://github.com/rootcause-rs/rootcause/pull/38)
- The `report_attachment!()` macro [#35](https://github.com/rootcause-rs/rootcause/pull/35)

### Changed

- Do more re-organization while it's free to do so [#37](https://github.com/rootcause-rs/rootcause/pull/37)

## [0.5.0] - 2025-10-27

### Changed

- Add a report header [#19](https://github.com/rootcause-rs/rootcause/pull/19)
- Make the `IteratorExt` trait more generic [#24](https://github.com/rootcause-rs/rootcause/pull/24)
- Rename `with_handler` to `custom` in most places [#25](https://github.com/rootcause-rs/rootcause/pull/25)

### Added

- Add a CHANGELOG [#27](https://github.com/rootcause-rs/rootcause/pull/27)

## [0.4.3] - 2025-10-22

### Changed

- Change the logo [#17](https://github.com/rootcause-rs/rootcause/pull/17)

### Added

- Add a discord badge [#16](https://github.com/rootcause-rs/rootcause/pull/16)

## [0.4.2] - 2025-10-22

### Changed

- Use `rustc-hash` instead of `foldhash` in the internal hooks [#15](https://github.com/rootcause-rs/rootcause/pull/15)

### Added

- Add the logo to the docs [#14](https://github.com/rootcause-rs/rootcause/pull/14)

## [0.4.1] - 2025-10-22

### Added

- Add a logo [#13](https://github.com/rootcause-rs/rootcause/pull/13)

## [0.4.0] - 2025-10-17

### Changed

- Various refactoring [#7](https://github.com/rootcause-rs/rootcause/pull/7)

## [0.3.0] - 2025-10-15

### Added

- Add CI and more documentation

### Changed

- Various refactoring

## [0.2.0] - 2025-10-07

### Added

- Initial release

[Unreleased]: https://github.com/rootcause-rs/rootcause/compare/v0.13.0...HEAD
[0.13.0]: https://github.com/rootcause-rs/rootcause/compare/v0.12.1...v0.13.0
[0.12.1]: https://github.com/rootcause-rs/rootcause/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/rootcause-rs/rootcause/compare/v0.11.1...v0.12.0
[0.11.1]: https://github.com/rootcause-rs/rootcause/compare/v0.11.0...v0.11.1
[0.11.0]: https://github.com/rootcause-rs/rootcause/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/rootcause-rs/rootcause/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/rootcause-rs/rootcause/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/rootcause-rs/rootcause/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/rootcause-rs/rootcause/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/rootcause-rs/rootcause/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/rootcause-rs/rootcause/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/rootcause-rs/rootcause/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/rootcause-rs/rootcause/compare/v0.4.3...v0.5.0
[0.4.3]: https://github.com/rootcause-rs/rootcause/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/rootcause-rs/rootcause/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/rootcause-rs/rootcause/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/rootcause-rs/rootcause/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/rootcause-rs/rootcause/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/rootcause-rs/rootcause/releases/tag/v0.2.0
