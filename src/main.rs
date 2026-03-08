use std::error::Error;

//use signal_hook::{consts::SIGINT, iterator::Signals};
use wlz::wlz::WlzServer;

/*
fn run() -> Result<(), Box<dyn Error>> {
    /*
    let display = Arc::new(server.display());
    let socket = display.add_socket_auto().ok_or("Failed to create socket")?;

    println!("Running compositor on socket: {socket}");
    println!("Starting compositor, C-c to exit");

    let term_disp = Arc::clone(display);

    let mut signals = Signals::new([SIGINT])?;

    let term_thread = thread::spawn(move || {
        // Should probably loop through this?
        if signals.forever().next().is_some() {
            term_disp.terminate();
        }
    });

    disp.run();

    term_thread.join().ok()?;

    Some(())
    */
    Ok(())
}
*/

fn main() -> Result<(), Box<dyn Error>> {
    #[allow(unused)]
    let server = WlzServer::try_create()?;
    Ok(())
}
