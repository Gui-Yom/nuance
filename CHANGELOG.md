# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Color picker for `layout(color) vec3`
- Triple drag value for `vec3`
- Grid to display sliders
- Manual documenting shader syntax
- Links to repo and manual

### Changed

- Split more things in module, major code refactor
- More error handling
- Ensure std430 alignment for Globals struct

### Fixed

- Correctly set params buffer size
- Unwatch old shader when loading a new one
- Do not create a buffer binding when there is no params (no buffer with size 0)

## [0.2.0] - 2021-05-12

### Added

- Load a shader with the load button
- Checkbox to activate fs watch

### Removed

- Load shader through cli

## [0.1.0] - 2021-05-11

### Added

- Initial release, more like a proof of concept
- Load shaders, preprocess some special directives
- GUI with egui

[Unreleased]: https://github.com/Gui-Yom/nuance/compare/v0.2.0...HEAD

[0.2.0]: https://github.com/Gui-Yom/nuance/compare/v0.1.0...v0.2.0

[0.1.0]: https://github.com/Gui-Yom/nuance/releases/tag/v0.1.0
