# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- WGPU power preference can be controlled with the env var WGPU_POWER_PREF, the -H switch has priority
- WGPU backend can be controlled with the env var WGPU_BACKEND
- The window and the side panel are now resizable, the canvas will be resized accordingly

[Unreleased]: https://github.com/Gui-Yom/nuance/compare/v0.3.2...HEAD

## [0.3.2]

### Fixed

- Use texture format Bgra8UnormSrgb (fixes #1)

[0.3.2]: https://github.com/Gui-Yom/nuance/compare/v0.3.1...v0.3.2

## [0.3.1] - 2021-08-31

### Added

- Add tooltips to the UI
- Add a manual reload button

### Changed

- Code structure has been refactored heavily

### Fixed

- No more warning when accessing a vector parameter component
- Dx12 is no more the forced wgpu backend (even on linux), that was a bug

[0.3.1]: https://github.com/Gui-Yom/nuance/compare/v0.3.0...v0.3.1

## [0.3.0] - 2021-07-25

### Added

- Color picker for `layout(color) vec3` param
- Triple drag value for `vec3` param
- Double drag value for `vec2` param
- Checkbox for `boolean` param
- Initializers for vector types
- Grid to display sliders
- Manual documenting shader syntax
- Links to repo and manual
- Reset button to reset params to their default values
- Initial support for WGSL shaders
- Pause shader execution with a new Pause button
- Access last shader execution result with samplePrevious()

### Changed (internal)

- Split more things into modules, major code refactoring
- More error handling
- Use mint types everywhere

### Fixed

- Correctly set params buffer size
- Unwatch old shader when loading a new one
- Do not create a buffer binding when there is no params (no buffer with size 0)
- Ensure std430 alignment for Globals struct with crevice
- Ensure std140 alignment for Params struct with crevice

[0.3.0]: https://github.com/Gui-Yom/nuance/compare/v0.2.0...v0.3.0

## [0.2.0] - 2021-05-12

### Added

- Load a shader with the load button
- Checkbox to activate fs watch

### Removed

- Load shader through cli

[0.2.0]: https://github.com/Gui-Yom/nuance/compare/v0.1.0...v0.2.0

## [0.1.0] - 2021-05-11

### Added

- Initial release, more like a proof of concept
- Load shaders, preprocess some special directives
- GUI with egui

[0.1.0]: https://github.com/Gui-Yom/nuance/releases/tag/v0.1.0
