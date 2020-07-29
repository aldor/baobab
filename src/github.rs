use anyhow::Context;
use serde::Deserialize;

const LINK_HEADER_NAME: &str = "Link";

trait LogResponse<T> {
    fn unwrap_200(self) -> anyhow::Result<T>;
}

impl LogResponse<reqwest::blocking::Response> for reqwest::Result<reqwest::blocking::Response> {
    fn unwrap_200(self) -> anyhow::Result<reqwest::blocking::Response> {
        let response = self.with_context(|| "request failed")?;
        let status = response.status();
        log::info!("response to {} got {}", response.url(), status);
        if status != 200 {
            let body_log = match response.text() {
                Ok(body) => format!("body: {}", body),
                Err(err) => format!("failed to read body: {}", err),
            };
            return Err(anyhow::anyhow!(
                "request failed, status: {}, {}",
                status,
                body_log
            ));
        }
        Ok(response)
    }
}

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

    /// Get open pull requests by user
    ///
    /// # Examples:
    ///
    /// ```
    /// let github_client = github::Client::new(env::var("GITHUB_HOST").unwrap().to_owned());
    /// let mut resp = github_client
    ///     .get_open_user_pull_requests("seanchaidh")
    ///     .unwrap();
    /// print!("{:?}\n", resp.response);
    /// print!("Link: {:?}\n", resp.link);
    /// loop {
    ///     for issue in resp.issues() {
    ///         println!("{:?}", issue);
    ///     }
    ///     resp = match resp.next_page() {
    ///         None => break,
    ///         Some(result) => result.expect("failed to get prs"),
    ///     };
    /// }
    /// ```

    pub fn get_open_user_pull_requests(
        &self,
        username: &str,
    ) -> anyhow::Result<IssuesSearchResult> {
        let url = format!(
            "{}/api/v3/search/issues?q=author:{}+is:open+is:pr+archived:false",
            self.host, username
        );
        let response = self
            .request(reqwest::Method::GET, &url)
            .send()
            .unwrap_200()?;
        self.parse_issues_request(response)
    }

    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::blocking::RequestBuilder {
        log::info!("requesting {} {}", method, url);
        self.client.request(method, url)
    }

    fn parse_issues_request(
        &self,
        response: reqwest::blocking::Response,
    ) -> anyhow::Result<IssuesSearchResult> {
        let link: Option<String> = match response.headers().get(LINK_HEADER_NAME) {
            Some(header) => Some(
                header
                    .to_str()
                    .with_context(|| format!("failed to parse {} header", LINK_HEADER_NAME))?
                    .to_owned(),
            ),
            None => None,
        };
        let body = response.text().with_context(|| "failed to parse body")?;
        let response: IssuesResponse =
            serde_json::from_str(&body).with_context(|| "failed to parse issues response body")?;
        Ok(IssuesSearchResult {
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

pub struct IssuesSearchResult<'a> {
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
            next = Some(url.to_owned());
        }
        if parts[1] == "rel=\"prev\"" {
            prev = Some(url.to_owned());
        }
    }
    Link { prev, next }
}

impl<'a> IssuesSearchResult<'a> {
    pub fn issues(&self) -> &Vec<Issue> {
        &self.response.items
    }

    pub fn next_page(self) -> Option<anyhow::Result<IssuesSearchResult<'a>>> {
        let url = match self.link {
            Some(Link {
                prev: _,
                next: Some(url),
            }) => url,
            _ => return None,
        };
        let response = self.client.client.get(&url).send().unwrap_200();
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
