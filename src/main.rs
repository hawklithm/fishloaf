mod app;
// #[cfg(feature = "crossterm")]
mod crossterm;
// #[cfg(feature = "termion")]
// mod termion;
mod ui;

mod client;
mod utils;

// #[cfg(feature = "crossterm")]
use crate::crossterm::run;
// #[cfg(feature = "termion")]
// use crate::termion::run;
use argh::FromArgs;
use std::{error::Error, time::Duration};

/// Demo
#[derive(Debug, FromArgs)]
struct Cli {
    /// time in ms between two ticks.
    #[argh(option, default = "250")]
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    #[argh(option, default = "true")]
    enhanced_graphics: bool,

    /// address of server or deamon process , default is 127.0.0.1
    #[argh(option, default = "String::from(\"127.0.0.1\")")]
    target: String,

    /// push notification port, default value is 9021
    #[argh(option, default = "9022")]
    push_notification_port: u16,

    ///  normal message connection port, default value is 9022
    #[argh(option, default = "9021")]
    normal_message_port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli: Cli = argh::from_env();
    let tick_rate = Duration::from_millis(cli.tick_rate);
    run(
        tick_rate,
        cli.enhanced_graphics,
        &cli.target,
        (cli.push_notification_port, cli.normal_message_port),
    )?;
    Ok(())
}
