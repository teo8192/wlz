use std::{sync::Arc, thread};

use wlz::wl::WlDisplay;
use signal_hook::{consts::SIGINT, iterator::Signals};

fn run() -> Option<()> {
    let disp = Arc::new(WlDisplay::try_create()?);
    let socket = disp.add_socket_auto()?;

    println!("Running compositor on socket: {socket}");
    println!("Starting compositor, C-c to exit");

    let term_disp = Arc::clone(&disp);

    let mut signals = Signals::new([SIGINT]).ok()?;

    let term_thread = thread::spawn(move || {
        // Should probably loop through this?
        if signals.forever().next().is_some() {
            term_disp.terminate();
        }
    });

    disp.run();

    term_thread.join().ok()?;

    Some(())
}

fn main() {
    if run().is_none() {
        eprintln!("Failed during execution of wayland compositor!");
    }
}
