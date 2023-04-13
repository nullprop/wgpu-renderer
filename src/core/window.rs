use super::state::State;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(debug_assertions)]
fn create_window(event_loop: &EventLoop<()>) -> winit::window::Window {
    use winit::dpi::PhysicalSize;
    WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1280, 720))
        .with_maximized(false)
        .build(event_loop)
        .unwrap()
}

#[cfg(not(debug_assertions))]
fn create_window(event_loop: &EventLoop<()>) -> winit::window::Window {
    WindowBuilder::new()
        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        .with_maximized(true)
        .build(event_loop)
        .unwrap()
}

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = create_window(&event_loop);

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        // https://github.com/rust-windowing/winit/pull/2074
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(1920, 1080));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                let canvas = web_sys::Element::from(window.canvas());
                body.append_child(&canvas).ok()
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = State::new(&window).await;
    let mut last_render = instant::Instant::now();
    let mut is_focused = true;

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::DeviceEvent { ref event, .. } => {
                state.input(None, Some(event));
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(Some(event), None) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        WindowEvent::Focused(focused) => {
                            lock_cursor(&window, *focused);
                            is_focused = *focused;
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = instant::Instant::now();
                let dt = now - last_render;
                last_render = now;
                if is_focused {
                    state.update(dt);
                    match state.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn lock_cursor(window: &winit::window::Window, lock: bool) {
    if lock {
        window
            .set_cursor_grab(if cfg!(target_arch = "wasm32") {
                winit::window::CursorGrabMode::Locked
            } else {
                winit::window::CursorGrabMode::Confined
            })
            .unwrap();
        window.set_cursor_visible(false);
    } else {
        window
            .set_cursor_grab(winit::window::CursorGrabMode::None)
            .unwrap();
        window.set_cursor_visible(true);
    }
}
