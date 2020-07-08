mod args;
mod config;
mod teamcity;

use std::thread;
use std::time;

use crossbeam;
use crossbeam_channel;
use indicatif;
use log::LevelFilter;
use log4rs;
use log4rs::append::rolling_file::policy::compound;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Config, Logger, Root};

fn main() {
    let args = args::parse_args();
    let build_url = args.url;
    init_logging();
    let config = config::read_config().expect("failed to read baobab config");
    let build_request =
        teamcity::BuildRequest::from_ui_url(&build_url).expect("failed to parse build url");
    let client = teamcity::Client::new(build_request.api_url.clone(), &config);
    crossbeam::thread::scope(|scope| {
        let (sender, receiver) = crossbeam_channel::unbounded();
        scope.spawn(move |_| print_progress(receiver));
        scope.spawn(move |_| {
            loop {
                // TODO: replace it with something like build.update(Self, client) -> Build?
                let build = client
                    .get_build(build_request.build_id)
                    .expect("failed to get build"); // TODO: add retries
                if build.state == "finished" {
                    break;
                }
                sender.send(build).unwrap();
                thread::sleep(time::Duration::from_secs(1));
            }
        });
    })
    .unwrap();
}

fn print_progress(build_channel: crossbeam_channel::Receiver<teamcity::Build>) {
    let progress_bar = indicatif::ProgressBar::new(100);
    set_style(&progress_bar, None);
    loop {
        let build = build_channel.recv().unwrap();
        set_style(&progress_bar, Some(&build));
        match &build.running_info {
            Some(info) => {
                progress_bar.set_message(&info.current_stage_text.as_str());
                progress_bar.set_position(build.percentage_complete.unwrap_or(100) as u64)
            }
            None => {
                progress_bar
                    .set_message(&format!("build is {} ({})", build.status, build.state).as_str());
                progress_bar.set_position(build.percentage_complete.unwrap_or(100) as u64);
            }
        };
        if build.state == "finished" {
            break;
        }
    }
}

fn set_style(progress_bar: &indicatif::ProgressBar, build: Option<&teamcity::Build>) {
    let color: &str;
    match build {
        Some(build) => {
            if build.status == "SUCCESS" {
                color = "green";
            } else if build.status == "FAILURE" {
                color = "red";
            } else {
                color = "gray";
            }
        }
        None => {
            color = "green";
        }
    }
    let template = format!(
        "{{spinner:.{}}} [{{elapsed_precise}}] [{{bar:40.cyan/blue}}] ({{eta}})\n{{wide_msg}}",
        &color
    );
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(&template)
            .progress_chars("#>-"),
    );
}

fn init_logging() {
    let file_path = "/tmp/foo.log";
    let file_path_pattern = "/tmp/foo.{}.log";
    let roller = compound::roll::fixed_window::FixedWindowRoller::builder()
        .build(file_path_pattern, 2)
        .expect("failed to init logs roller");
    let rotate_policy = compound::CompoundPolicy::new(
        Box::new(compound::trigger::size::SizeTrigger::new(20 * 1024 * 1024)),
        Box::new(roller),
    );
    let logfile = RollingFileAppender::builder()
        .build(file_path, Box::new(rotate_policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(
            Logger::builder()
                .appender("logfile")
                .build("app::backend::db", LevelFilter::Trace),
        )
        .build(Root::builder().appender("logfile").build(LevelFilter::Warn))
        .unwrap();
    log4rs::init_config(config).unwrap();
}
