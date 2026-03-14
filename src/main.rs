use std::{error::Error, mem::MaybeUninit};

use std::pin::pin;

use wlz::{wlz::WlzServer, wrapper::log};

fn main() -> Result<(), Box<dyn Error>> {
    log::init(log::LogLevel::Debug);

    // use pin!() for having it on stack and Box::pin for being on heap
    let mut server = pin!(MaybeUninit::uninit());
    let _server = WlzServer::initialize(server.as_mut())?;

    Ok(())
}
