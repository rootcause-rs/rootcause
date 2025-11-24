# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] - Unreleased

### Changed

- Fix issue [#64](https://github.com/rootcause-rs/rootcause/issues/64) by removing a lot of trait bounds [#67](https://github.com/rootcause-rs/rootcause/pull/67)

## [0.9.1] - 2025-11-23

### Fixed

- Fixed the building of docs on docs.rs [#65](https://github.com/rootcause-rs/rootcause/pull/65)

## [0.9.0] - 2025-11-22

### Added

- Added a new `compat` module added poulated it with `eyre` and `error-stack` compatibility [#55](https://github.com/rootcause-rs/rootcause/pull/55)
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

- More docs, examples, README [#40](https://github.com/rootcause-rs/rootcause/pull/40)
- More docs, examples, README [#41](https://github.com/rootcause-rs/rootcause/pull/41)

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

[0.10.0]: https://github.com/rootcause-rs/rootcause/compare/v0.9.1...HEAD
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
