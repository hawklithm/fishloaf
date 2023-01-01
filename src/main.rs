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
use time::{format_description, UtcOffset};
use tracing::{dispatcher, Dispatch, Level, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::time::OffsetTime, FmtSubscriber};

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

    ///  log file home path
    #[argh(
        option,
        default = "String::from(dirs::home_dir().unwrap().to_str().unwrap())"
    )]
    log_home: String,
}

fn make_dispatch(log_file_director: &str) -> (Dispatch, WorkerGuard) {
    let file_appender = tracing_appender::rolling::daily(log_file_director, "common-default.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let zone_offset = match UtcOffset::current_local_offset() {
        Ok(t) => t,
        Err(e) => {
            println!("UtcOffset::current_local_offset in make_dispatch error! use +8 zone as default! message={:?}",e);
            UtcOffset::from_hms(8, 0, 0).unwrap()
        }
    };
    let timer = OffsetTime::new(
        zone_offset,
        format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]",
        )
        .unwrap(),
    );
    let subscriber = FmtSubscriber::builder()
        .with_level(true)
        .with_timer(timer)
        .with_file(false)
        .with_thread_names(false)
        .with_thread_ids(true)
        .with_ansi(false)
        .with_target(false)
        .with_max_level(Level::INFO)
        .with_writer(non_blocking)
        .finish();
    (Dispatch::new(subscriber), guard)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli: Cli = argh::from_env();
    let tick_rate = Duration::from_millis(cli.tick_rate);

    let (dispatcher, guard) = make_dispatch(&format!("{}/{}", &cli.log_home, "fishloaf"));
    dispatcher::with_default(&dispatcher, || {
        info!("fishloaf termchat start!");
        run(
            tick_rate,
            cli.enhanced_graphics,
            &cli.target,
            (cli.push_notification_port, cli.normal_message_port),
        )
    })?;

    Ok(())
}
