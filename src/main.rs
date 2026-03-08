use std::error::Error;

use wlz::{wlz::WlzServer, wrapper::log};

fn main() -> Result<(), Box<dyn Error>> {
    log::init(log::LogLevel::Debug);

    #[allow(unused)]
    let server = WlzServer::try_create()?;
    Ok(())
}
