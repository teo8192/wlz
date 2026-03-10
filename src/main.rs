use std::{error::Error, mem::MaybeUninit};

use wlz::{wlz::WlzServer, wrapper::log};

fn main() -> Result<(), Box<dyn Error>> {
    log::init(log::LogLevel::Debug);

    let server = Box::pin(MaybeUninit::uninit());
    let mut server = WlzServer::initialize(server)?;
    // Todo: maybe only work on Pin<&mut WlzServer> to run and so on?
    let _server = unsafe { server.as_mut().get_unchecked_mut() };

    Ok(())
}
