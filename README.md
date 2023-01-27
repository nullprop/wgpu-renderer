# wpgu-renderer

A small [wgpu](https://github.com/gfx-rs/wgpu) renderer written in [Rust](https://github.com/rust-lang/rust).

## Features

- Physically based shading
  - (F: Fresnel-Schlick approximation)
  - (G: Smith's Schlick-GGX)
  - (D: Trowbridge-Reitz GGX)
- gltf models
- 1 realtime light
- Simple wgsl preprocessor for includes

## References
- [Learn Wgpu](https://sotrh.github.io/learn-wgpu/)
- [Learn OpenGL](https://learnopengl.com/)
