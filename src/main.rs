mod core;
mod shaders;

fn main() {
    env_logger::init();
    pollster::block_on(core::window::run());
}
