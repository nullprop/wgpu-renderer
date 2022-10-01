mod surf;

fn main() {
    env_logger::init();
    pollster::block_on(surf::updater::run());
}

