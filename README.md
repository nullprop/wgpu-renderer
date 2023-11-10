# wpgu-renderer

A small [wgpu](https://github.com/gfx-rs/wgpu) renderer written in [Rust](https://github.com/rust-lang/rust).

![](demo.gif)

## Try it out
[rasanen.dev/wgpu-renderer](https://rasanen.dev/wgpu-renderer)  
Note: the .wasm is about 50 MB because it embeds Sponza.  
Note: shadowmaps and volumetric fog are not enabled on WebGL.

Controls:
- WASD - Move horizontally
- Ctrl/Space - Move vertically
- Mouse - Look around
- Scrollwheel - Increase/Decrease movement speed
- ESC - Quit (Only on standalone version)

## Features

- PBS
- glTF models
- 1 realtime pointlight
- Shadow mapping & PCF*
- Volumetric fog*
- Simple wgsl preprocessor for includes
- Runs on WASM and native desktop
  - Tested on:
    - `Ubuntu 22.04 (Mesa 23.1.0-devel)`
    - `Windows 10 Pro 21H2`
    - `Firefox 109.0`
    - `Chrome 109.0.5414.120`

*Native only

TODO:
- Get shadowmaps & volumetric fog to work on WebGL?
- Improve shadowmap artifacts
- Transparency
- Refactor
    - Simplify/abstract renderpasses
    - `src/core/state.rs` is a mess
- SSAO
- Bloom
- AA
- Texture filtering
- Immediate mode UI (dear imgui, egui)
- Some type of GI (DDGI, VXGI)

## Running locally

Standalone:
```sh
cargo run --release
```

WASM requires:
- [wasm-pack](https://github.com/rustwasm/wasm-pack)
  - `cargo install wasm-pack`
- [miniserve](https://github.com/svenstaro/miniserve), or some other http server, such as `python3 -m http.server`.
  - For miniserve, see: `run-wasm.sh`

## References
- [wgpu examples](https://github.com/gfx-rs/wgpu/blob/master/wgpu/examples)
- [Learn Wgpu](https://sotrh.github.io/learn-wgpu/)
- [Learn OpenGL: PBR](https://learnopengl.com/PBR/Theory)
- [42yeah: Raymarching Clouds](https://blog.42yeah.is/rendering/2023/02/11/clouds.html)

## Assets
- Sponza
  - Obtained from [KhronosGroup glTF-Sample-Models repository](https://github.com/KhronosGroup/glTF-Sample-Models/tree/16e2408d31e06d4b0bcf6123db472e802d71f081/2.0/Sponza), converted to .glb
- Cube
  - Obtained from [KhronosGroup glTF-Sample-Models repository](https://github.com/KhronosGroup/glTF-Sample-Models/tree/16e2408d31e06d4b0bcf6123db472e802d71f081/2.0/Cube), converted to .glb
