use super::state::State;
use winit::{
    event::*,
    event_loop::{EventLoop},
    window::WindowBuilder,
};
use winit::keyboard::{KeyCode, PhysicalKey};

#[cfg(debug_assertions)]
fn create_window(event_loop: &EventLoop<()>) -> winit::window::Window {
    log::info!("Creating window");
    use winit::dpi::PhysicalSize;
    WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1280, 720))
        .with_maximized(false)
        .build(event_loop)
        .unwrap()
}

#[cfg(not(debug_assertions))]
fn create_window(event_loop: &EventLoop<()>) -> winit::window::Window {
    log::info!("Creating window");
    WindowBuilder::new()
        .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        .with_maximized(true)
        .build(event_loop)
        .unwrap()
}

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    let window = create_window(&event_loop);

    #[cfg(target_arch = "wasm32")]
    {
        log::info!("Appending canvas to document");
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                let canvas = web_sys::Element::from(window.canvas().unwrap());
                body.append_child(&canvas).ok()
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = State::new(&window).await;
    let mut last_render = instant::Instant::now();
    let start_time = instant::Instant::now();
    let mut is_focused = true;

    // Event loop
    event_loop.run(move |event, window_target| {
        match event {
            Event::DeviceEvent { ref event, .. } => {
                state.input(None, Some(event));
            }
            // window render
            Event::WindowEvent { window_id, event: WindowEvent::RedrawRequested }
            if window_id == window.id() => {
                let now = instant::Instant::now();
                let dt = now - last_render;
                let time = now - start_time;
                last_render = now;
                if is_focused {
                    state.update(dt, time);
                    match state.render() {
                        Ok(_) => {
                            window.request_redraw();
                        }
                        // Reconfigure the surface if lost
                        Err(wgpu::SurfaceError::Lost) => {
                            state.resize(state.size);
                            window.request_redraw();
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => window_target.exit(),
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => {
                            eprintln!("{:?}", e);
                            window.request_redraw();
                        }
                    }
                }
            }
            // misc window input
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(Some(event), None) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                            ..
                        } => {
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                window_target.exit();
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            log::info!("WindowEvent::Resized {}:{}", physical_size.width, physical_size.height);
                            state.resize(*physical_size);
                            window.request_redraw();
                        }
                        WindowEvent::Focused(focused) => {
                            lock_cursor(&window, *focused);
                            is_focused = *focused;
                            window.request_redraw();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }).unwrap();
}

fn lock_cursor(window: &winit::window::Window, lock: bool) {
    if lock {
        if let Err(e) = window
            .set_cursor_grab(if cfg!(target_arch = "wasm32") {
                winit::window::CursorGrabMode::Locked
            } else {
                winit::window::CursorGrabMode::Confined
            })
        {
            println!("Failed to grab cursor {e:?}")
        }
        window.set_cursor_visible(false);
    } else {
        window
            .set_cursor_grab(winit::window::CursorGrabMode::None)
            .unwrap();
        window.set_cursor_visible(true);
    }
}
