use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use lazy_static::lazy_static;
use reqwest::cookie::Jar;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::Url;
use serde::de::DeserializeOwned;

use super::error::Error;
use super::response::Response;
use super::types::IllustId;
use super::types::Profile;
use super::types::ProfileId;
use super::types::UgoiraMeta;

lazy_static! {
    pub static ref INSTANCE: Client = Client::new().unwrap();
}

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; rv:91.0) Gecko/20100101 Firefox/91.0";
const PIXIV_ROOT: &str = "https://www.pixiv.net/";

pub struct Client {
    jar: Arc<Jar>,
    http: reqwest::Client,
}

impl Client {
    fn new() -> Result<Self> {
        let default_headers = {
            let mut default_headers = HeaderMap::new();

            default_headers.insert(
                reqwest::header::REFERER,
                HeaderValue::from_static(PIXIV_ROOT),
            );
            default_headers.insert(
                reqwest::header::USER_AGENT,
                HeaderValue::from_static(USER_AGENT),
            );

            default_headers
        };

        let jar = Arc::new(Jar::default());
        let http = reqwest::Client::builder()
            .cookie_provider(Arc::clone(&jar))
            .default_headers(default_headers)
            .build()?;

        Ok(Self { jar, http })
    }

    pub fn login(&self, cookie: impl AsRef<str>) {
        let cookie = cookie.as_ref();
        let cookie = format!("PHPSESSID={cookie}");
        let url = Url::parse(PIXIV_ROOT).unwrap();
        self.jar.add_cookie_str(&cookie, &url)
    }

    pub async fn profile(&self, id: ProfileId) -> Result<Profile> {
        let url = format!("{PIXIV_ROOT}ajax/user/{id}/profile/all");
        self.get(url).await
    }

    pub async fn ugoira_meta(&self, id: IllustId) -> Result<UgoiraMeta> {
        let url = format!("{PIXIV_ROOT}ajax/illust/{id}/ugoira_meta");
        self.get(url).await
    }

    pub async fn download_ugoira<'a>(&'a self, meta: &UgoiraMeta) -> Result<Bytes> {
        let original_src = meta.original_src.clone();
        let resp = self.http.get(original_src).send().await?;
        let data = resp.bytes().await?;

        Ok(data)
    }

    async fn get<T, S>(&self, url: S) -> Result<T>
    where
        T: DeserializeOwned,
        S: AsRef<str>,
    {
        let response = self.http.get(url.as_ref()).send().await?;

        let data = response.bytes().await?;
        let json = serde_json::from_slice(&data)?;

        match json {
            Response {
                error: true,
                message,
                ..
            } => Err(Error::Pixiv(message).into()),
            Response { body: None, .. } => Err(Error::NoData.into()),
            Response {
                body: Some(body), ..
            } => Ok(body),
        }
    }
}
