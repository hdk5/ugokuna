use anyhow::Result;
use bytes::Bytes;
use lazy_static::lazy_static;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
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

pub struct Client {
    http: reqwest::Client,
}

impl Client {
    fn new() -> Result<Self> {
        let default_headers = {
            let mut default_headers = HeaderMap::new();

            default_headers.insert(
                reqwest::header::REFERER,
                HeaderValue::from_static("https://www.pixiv.net/"),
            );
            default_headers.insert(
                reqwest::header::USER_AGENT,
                HeaderValue::from_static("PixivIOSApp/7.13.3 (iOS 14.6; iPhone13,2)"),
            );

            default_headers
        };

        let http = reqwest::Client::builder()
            .default_headers(default_headers)
            .build()?;

        Ok(Self { http })
    }

    pub async fn profile(&self, id: ProfileId) -> Result<Profile> {
        let url = format!("https://www.pixiv.net/ajax/user/{}/profile/all", id);
        self.get(url).await
    }

    pub async fn ugoira_meta(&self, id: IllustId) -> Result<UgoiraMeta> {
        let url = format!("https://www.pixiv.net/ajax/illust/{}/ugoira_meta", id);
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
