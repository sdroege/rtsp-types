# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html),
specifically the [variant used by Rust](http://doc.crates.io/manifest.html#the-version-field).

## [Unreleased]

## [0.0.3]- 2021-09-24
### Changed
- Updated to nom 7.

## [0.0.2]- 2021-06-05
### Fixed
- Re-export `url::Host` as it's used in the API.
- Fix build on 32 bit platforms.
- Don't panic on bad `Content-Length` headers.

### Added
- Add typed headers for various RTSP headers.
- `cargo-fuzz` integration.

## 0.0.1 - 2020-11-13
- Initial release of the `rtsp-types` crate.

[Unreleased]: https://github.com/sdroege/rtsp-types/compare/0.0.3...HEAD
[0.0.3]: https://github.com/sdroege/rtsp-types/compare/0.0.2...0.0.3
[0.0.2]: https://github.com/sdroege/rtsp-types/compare/0.0.1...0.0.2
