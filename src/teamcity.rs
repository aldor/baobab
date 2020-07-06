use crate::config;

use log;
use serde;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub status: String,
    pub state: String, // TODO: enums
    pub percentage_complete: Option<i64>,
    pub web_url: String,
    #[serde(rename = "running-info")]
    pub running_info: Option<RunningInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningInfo {
    pub elapsed_seconds: i64,
    pub estimated_total_seconds: i64,
    pub current_stage_text: String,
}

#[derive(Debug)]
pub struct Client<'a> {
    api_url: String,
    client: reqwest::blocking::Client,
    // TODO: pass auth options separately
    config: &'a config::Config,
}

impl Client<'_> {
    pub fn new(api_url: String, config: &config::Config) -> Client {
        Client {
            api_url,
            client: reqwest::blocking::Client::new(),
            config,
        }
    }

    pub fn get_build(&self, build_id: u64) -> Result<Build, String> {
        let url = format!("{}/app/rest/builds/id:{}", self.api_url, build_id);
        let resp = match self
            .client
            .get(&url)
            .basic_auth(
                self.config.teamcity_username.as_str(),
                Some(self.config.teamcity_password.as_str()),
            )
            .header("Accept", "application/json")
            .send()
        {
            Ok(resp) => resp,
            Err(error) => return Err(format!("request failed: {}", error)),
        };
        let body = match resp.text() {
            Ok(text) => text,
            Err(error) => return Err(format!("failed to get resp body: {}", error)),
        };
        log::debug!("got build {}", body);
        // println!("{}", body);
        let build: Build = match serde_json::from_str(body.as_str()) {
            Ok(build) => build,
            Err(error) => return Err(format!("failed to parse build: {}", error)),
        };
        Ok(build)
    }
}
#[derive(Debug)]
pub struct BuildRequest {
    pub build_id: u64,
    pub api_url: String,
}

impl BuildRequest {
    pub fn from_ui_url(url: &str) -> Result<BuildRequest, String> {
        let parsed_url = match reqwest::Url::parse(url) {
            Ok(url) => url,
            Err(error) => return Err(format!("failed to parse url: {}", error)),
        };
        let host = match parsed_url.host_str() {
            Some(host) => host,
            None => return Err("failed to read host from url".to_string()),
        };
        let api_url = format!("{}://{}", parsed_url.scheme(), host);
        let build_id_param_name = "buildId";
        for pair in parsed_url.query_pairs() {
            if pair.0 == build_id_param_name {
                let id: u64 = match pair.1.parse() {
                    Ok(id) => id,
                    Err(error) => {
                        return Err(format!(
                            "failed to parse {}: {}",
                            build_id_param_name, error
                        ));
                    }
                };
                return Ok(BuildRequest {
                    build_id: id,
                    api_url,
                });
            }
        }
        Err(format!("{} not found in url", build_id_param_name))
    }
}
