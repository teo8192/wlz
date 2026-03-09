use std::error::Error;

use wlz::{wlz::WlzServer, wrapper::log};

fn main() -> Result<(), Box<dyn Error>> {
    log::init(log::LogLevel::Debug);

    let mut server = unsafe { WlzServer::uninitialized() };
    let server = server.as_mut();
    server.initialize()
}
