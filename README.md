# Shadertoy
(Will be) a desktop equivalent to https://shadertoy.com.

Currently a good demo for wgpu-rs

### Shaders
Shaders are written with GLSL (`#version 460`).

### TODO
 - Live reloading shaders
 - Bind uniforms
   - time in milliseconds (iTime)
   - screen resolution (vec3 Resolution (width, height, ratio))
   - custom values ?
 - Bind resources (like textures)
 - CLI (clap)
 - GUI (imgui)
