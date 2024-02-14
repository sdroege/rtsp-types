# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html),
specifically the [variant used by Rust](http://doc.crates.io/manifest.html#the-version-field).

## [Unreleased]

## [0.1.1]- 2024-02-14
### Fixed
- Fix numeric value of `InvalidRange` status code.

### Changed
- Declare and check MSRV on the CI.

## [0.1.0]- 2023-06-30
### Fixed
- Support mode without quotes in the transport header and make it case
  insensitive.

### Changed
- Return minimally required length when parsing fails because of an incomplete
  message.

## [0.0.5]- 2023-02-02
### Fixed
- Trim whitespace from header values in accordance to RFC9110.

## [0.0.4]- 2022-10-27

### Fixed
- Parsing of the optional timeout field of the `Session` header.

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

[Unreleased]: https://github.com/sdroege/rtsp-types/compare/0.1.1...HEAD
[0.1.1]: https://github.com/sdroege/rtsp-types/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/sdroege/rtsp-types/compare/0.0.5...0.1.0
[0.0.5]: https://github.com/sdroege/rtsp-types/compare/0.0.4...0.0.5
[0.0.4]: https://github.com/sdroege/rtsp-types/compare/0.0.3...0.0.4
[0.0.3]: https://github.com/sdroege/rtsp-types/compare/0.0.2...0.0.3
[0.0.2]: https://github.com/sdroege/rtsp-types/compare/0.0.1...0.0.2
