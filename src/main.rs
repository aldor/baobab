mod args;
mod config;
mod teamcity;

use std::thread;
use std::time;

use indicatif;

fn main() {
    let args = args::parse_args();
    let build_url = args.url;
    let config = config::read_config().expect("failed to read baobab config");
    let build_request =
        teamcity::BuildRequest::from_ui_url(&build_url).expect("failed to parse build url");
    let client = teamcity::Client::new(build_request.api_url, &config);
    let progress_bar = indicatif::ProgressBar::new(100);
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg} ({eta})")
            .progress_chars("#>-"),
    );
    loop {
        // TODO: replace it with something like build.update(Self, client) -> Build?
        let build = client
            .get_build(build_request.build_id)
            .expect("failed to get build"); // TODO: add retries
        match build.running_info {
            Some(info) => {
                progress_bar.set_message(info.current_stage_text.as_str());
                progress_bar.set_position(build.percentage_complete.unwrap_or(100) as u64)
            }
            None => {
                progress_bar
                    .set_message(format!("build is {} ({})", build.status, build.state).as_str());
                progress_bar.set_position(build.percentage_complete.unwrap_or(100) as u64);
            }
        };
        thread::sleep(time::Duration::from_secs(1));
    }
}
