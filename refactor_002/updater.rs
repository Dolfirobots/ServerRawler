use serde::Deserialize;
use reqwest::{ Client, header::USER_AGENT };
use chrono::{ DateTime, Utc };
use semver::Version;

#[derive(Debug, Clone, Deserialize)]
pub struct GithubRelease {
    #[serde(rename = "tag_name")]
    pub version_tag: String,

    #[serde(rename = "name")]
    pub version_name: Option<String>,

    #[serde(rename = "prerelease")]
    pub pre_release: bool,

    pub body: Option<String>,

    #[serde(rename = "published_at")]
    pub date: DateTime<Utc>,
}

pub struct GithubAPI {
    username: String,
    repository: String,
    client: Client,
    agent: String,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ReleaseFilter {
    Release,
    Prerelease,
    All,
}

impl GithubAPI {
    /// Create a new GitHub config
    pub fn new(username: &str, repository: &str, agent: &str) -> Self {
        Self {
            username: username.to_string(),
            repository: repository.to_string(),
            client: Client::new(),
            agent: agent.to_string(),
        }
    }

    // Helper
    fn base_url(&self) -> String {
        format!("https://api.github.com/repos/{}/{}", self.username, self.repository)
    }

    /// Get latest GitHub release
    pub async fn get_latest_release(&self) -> Result<GithubRelease, reqwest::Error> {
        let url = format!("{}/releases/latest", self.base_url());

        let release: GithubRelease = self.client.get(&url)
            .header(USER_AGENT, "ServerRawler")
            .send().await?
            .json().await?;

        Ok(release)
    }

    // Get all versions and cache them
    pub async fn get_all_versions(&self) -> Result<Vec<GithubRelease>, reqwest::Error> {
        let url = format!("{}/releases", self.base_url());

        let releases: Vec<GithubRelease> = self.client.get(&url)
            .header(USER_AGENT, "ServerRawler")
            .send().await?
            .json().await?;

        Ok(releases)
    }

    /// Gets how much versions you are behind
    pub async fn get_behind(&self, current_version: &str, filter: ReleaseFilter) -> Result<usize, reqwest::Error> {
        let versions = self.get_all_versions().await?;
        Ok(self.calculate_behind(&versions, current_version, filter))
    }

    /// Calc the behind counter
    pub fn calculate_behind(&self, all_versions: &[GithubRelease], current_version: &str, filter: ReleaseFilter) -> usize {
        let mut count = 0;
        for release in all_versions {
            if release.version_tag == current_version {
                break;
            }
            // Filtering releases
            let should_count = match filter {
                ReleaseFilter::All => true,
                ReleaseFilter::Release => !release.pre_release,
                ReleaseFilter::Prerelease => release.pre_release,
            };

            if should_count {
                count += 1;
            }
        }
        count
    }

    /// Check, if version is the latest
    pub async fn check_version(&self, version_to_check: &str) -> Result<bool, reqwest::Error> {
        let latest = self.get_latest_release().await?;
        Ok(latest.version_tag == version_to_check)
    }
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