use std::{error::Error, mem::MaybeUninit};

use std::pin::pin;

use wlz::info;
use wlz::wlz::WlzServer;
use wlz::wrapper::log;

fn main() -> Result<(), Box<dyn Error>> {
    log::init(log::LogLevel::Debug);

    // use pin!() for having it on stack and Box::pin for being on heap
    let mut server = pin!(MaybeUninit::uninit());
    let mut server = WlzServer::initialize(server.as_mut())?;

    /* Add a Unix socket to the Wayland display. */
    let socket = server.as_mut().add_socket_auto()?;
    info!("Got socket: {}", socket);

    /* Start the backend. This will enumerate outputs and inputs, become the DRM
     * master, etc */
    server.as_mut().start_backend()?;

    // TODO setenv("WAYLAND_DISPLAY", socket, true);
    // and execute startup cmd ...

    /* Run the Wayland event loop. This does not return until you exit the
     * compositor. Starting the backend rigged up all of the necessary event
     * loop configuration to listen to libinput events, DRM events, generate
     * frame events at the refresh rate, and so on. */
    info!("Running Wayland compositor on WAYLAND_DISPLAY={}", socket);
    server.as_mut().run();

    Ok(())
}
