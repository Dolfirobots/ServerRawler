use reqwest::Client;
use semver::Version;
use serde::Deserialize;

pub struct GithubAPI {
    user: String,
    repo: String,

    agent: String,
    client: Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubVersion {
    pub tag_name: String,
    pub name: String,
    pub created_at: String,
}

impl GithubAPI {
    pub fn new(user: String, repo: String) -> Self {
        Self {
            user,
            repo,
            agent: "Rust Github Checker".to_string(),
            client: Client::new(),
        }
    }

    pub async fn get_latest_version(&self) -> Result<GithubVersion, reqwest::Error> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.user, self.repo
        );

        self.client
            .get(url)
            .header(reqwest::header::USER_AGENT, &self.agent)
            .send()
            .await?.json()
            .await
    }

    pub async fn get_behind(&self, current_tag: &str) -> Result<i32, reqwest::Error> {
        let versions = self.get_all_versions().await?;

        let current = parse_semver(current_tag);

        let mut behind = 0;

        for v in versions {
            if let (Some(latest), Some(curr)) = (parse_semver(&v.tag_name), current.clone()) {
                if latest > curr {
                    behind += 1;
                }
            }
        }

        Ok(behind)
    }

    pub async fn get_all_versions(&self) -> Result<Vec<GithubVersion>, reqwest::Error> {
        let mut page = 1;
        let mut all_versions = Vec::new();

        loop {
            let url = format!(
                "https://api.github.com/repos/{}/{}/releases?per_page=100&page={}",
                self.user, self.repo, page
            );

            let mut page_versions: Vec<GithubVersion> = self.client
                .get(&url)
                .header(reqwest::header::USER_AGENT, &self.agent)
                .send()
                .await?
                .json()
                .await?;

            if page_versions.is_empty() {
                break;
            }

            all_versions.append(&mut page_versions);
            page += 1;
        }

        all_versions.sort_by(|a, b| {
            parse_semver(&b.tag_name).cmp(&parse_semver(&a.tag_name))
        });

        Ok(all_versions)
    }


    pub fn set_agent(mut self, agent: String) -> Self {
        self.agent = agent;
        self
    }
}

fn parse_semver(tag: &str) -> Option<Version> {
    let cleaned = tag.trim_start_matches('v');
    Version::parse(cleaned).ok()
}

pub fn clean_version(version_str: String) -> String {
    match Version::parse(version_str.trim_start_matches('v')) {
        Ok(mut v) => {
            v.pre = semver::Prerelease::EMPTY;
            v.build = semver::BuildMetadata::EMPTY;
            v.to_string()
        }
        Err(_) => version_str.to_string(),
    }
}