mod core;

fn main() {
    env_logger::init();
    pollster::block_on(core::updater::run());
}
