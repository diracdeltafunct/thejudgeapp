use crate::models::{CardSummary, ScryfallObject};
use reqwest::Client;
use serde_json::Value;
use url::Url;

const DEFAULT_BASE_URL: &str = "https://api.scryfall.com";

#[derive(Debug)]
pub enum ScryfallError {
    Url(url::ParseError),
    Http(reqwest::Error),
    Parse(serde_json::Error),
    MissingField(&'static str),
}

impl From<reqwest::Error> for ScryfallError {
    fn from(err: reqwest::Error) -> Self {
        ScryfallError::Http(err)
    }
}

impl From<serde_json::Error> for ScryfallError {
    fn from(err: serde_json::Error) -> Self {
        ScryfallError::Parse(err)
    }
}

impl From<url::ParseError> for ScryfallError {
    fn from(err: url::ParseError) -> Self {
        ScryfallError::Url(err)
    }
}

#[derive(Clone, Debug)]
pub struct ScryfallClient {
    http: Client,
    base_url: Url,
}

impl ScryfallClient {
    pub fn new() -> Result<Self, ScryfallError> {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    pub fn with_base_url(base_url: &str) -> Result<Self, ScryfallError> {
        let http = Client::builder().build()?;
        let base_url = Url::parse(base_url)?;
        Ok(Self { http, base_url })
    }

    pub async fn card_named(&self, name: &str) -> Result<ScryfallObject, ScryfallError> {
        let mut url = self.base_url.join("cards/named")?;
        url.query_pairs_mut().append_pair("fuzzy", name);
        self.get_json(url).await
    }

    pub async fn card_search(&self, query: &str) -> Result<ScryfallObject, ScryfallError> {
        let mut url = self.base_url.join("cards/search")?;
        url.query_pairs_mut().append_pair("q", query);
        self.get_json(url).await
    }

    pub async fn card_rulings(&self, card_id: &str) -> Result<ScryfallObject, ScryfallError> {
        let url = self.base_url.join(&format!("cards/{card_id}/rulings"))?;
        self.get_json(url).await
    }

    pub async fn card_summary(&self, name: &str) -> Result<CardSummary, ScryfallError> {
        let card = self.card_named(name).await?;
        CardSummary::try_from(card)
    }

    async fn get_json(&self, url: Url) -> Result<ScryfallObject, ScryfallError> {
        let value = self.http.get(url).send().await?.json::<Value>().await?;
        Ok(ScryfallObject::Raw(value))
    }
}
