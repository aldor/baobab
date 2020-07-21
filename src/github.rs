use anyhow::Context;
use serde::Deserialize;

#[derive(Debug)]
pub struct Client {
    host: String,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn new(host: String) -> Client {
        Client {
            host,
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn search_open_user_pull_requests(&self, username: &str) -> anyhow::Result<IssuesRequest> {
        let response = self
            .client
            .get(&format!(
                "{}/api/v3/search/issues?q=author:{}",
                self.host, username
            ))
            .send()
            .with_context(|| "request for opened pull requests failed!")?;
        self.parse_issues_request(response)
    }

    fn parse_issues_request(
        &self,
        response: reqwest::blocking::Response,
    ) -> anyhow::Result<IssuesRequest> {
        let link: Option<String> = match response.headers().get("Link") {
            Some(header) => Some(
                header
                    .to_str()
                    .with_context(|| "failed to parse Link header")?
                    .to_owned(),
            ),
            None => None,
        };
        // let _link = link.map(parse_link);
        // let link = parse_link(link);
        let body = response
            .text()
            .with_context(|| "failed to parse pull requests search body")?;
        let response: IssuesResponse =
            serde_json::from_str(&body).with_context(|| "failed to parse body")?;
        Ok(IssuesRequest {
            client: &self,
            response,
            link: link.map(parse_link),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct IssuesResponse {
    total_count: u64,
    incomplete_results: bool,
    items: Vec<Issue>,
}

pub struct IssuesRequest<'a> {
    client: &'a Client,
    pub response: IssuesResponse,
    pub link: Option<Link>,
}

#[derive(Debug)]
pub struct Link {
    prev: Option<String>,
    next: Option<String>,
}

fn parse_link(link: String) -> Link {
    let mut prev: Option<String> = None;
    let mut next: Option<String> = None;
    for part in link.split(',') {
        let parts: Vec<&str> = part.split(';').map(|v| v.trim()).collect();
        assert!(parts.len() == 2);
        let url = &parts[0][1..parts[0].len() - 1];
        if parts[1] == "rel=\"next\"" {
            next = Some(url.to_string());
        }
        if parts[1] == "rel=\"prev\"" {
            prev = Some(url.to_string());
        }
    }
    Link { prev, next }
}

impl<'a> IssuesRequest<'a> {
    pub fn issues(&self) -> &Vec<Issue> {
        &self.response.items
    }

    pub fn next_page(self) -> Option<anyhow::Result<IssuesRequest<'a>>> {
        let url = match self.link {
            Some(Link {
                prev: _,
                next: Some(url),
            }) => url,
            _ => return None,
        };
        let response = self
            .client
            .client
            .get(&url)
            .send()
            .with_context(|| "request for opened pull requests failed!");
        match response {
            Ok(response) => Some(self.client.parse_issues_request(response)),
            Err(err) => Some(Err(err)),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    url: String,
    id: u64,
    title: String,
}
