# wpgu-renderer

A small [wgpu](https://github.com/gfx-rs/wgpu) renderer written in [Rust](https://github.com/rust-lang/rust).

## Features

- Physically based shading
  - (F: Fresnel-Schlick approximation)
  - (G: Smith's Schlick-GGX)
  - (D: Trowbridge-Reitz GGX)
- glTF models
- 1 realtime light
- Simple wgsl preprocessor for includes

Things I would like to add:
- Shadow mapping
- Restructuring
    - Simplify/abstract renderpasses; will be nice to have for PP and GI
    - `src/core/state.rs` is a mess; separate input handling, pipeline, passes
- SSAO
- Bloom
- Immediate mode UI (dear imgui, egui)
- Editing material properties, lights at runtime through UI
- Some type of GI (DDGI, VXGI)

## References
- [Learn Wgpu](https://sotrh.github.io/learn-wgpu/)
- [Learn OpenGL](https://learnopengl.com/)

## Assets
- Sponza
  - Obtained from [KhronosGroup glTF-Sample-Models repository], converted to .glb
