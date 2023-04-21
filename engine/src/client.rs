use crate::runtime::{PatternName, Response};
use crate::value::RuntimeValue;
use http::StatusCode;
use once_cell::sync::OnceCell;
use url::Url;

// might be unused on wasm32
#[allow(unused)]
fn user_agent() -> &'static str {
    static USER_AGENT: OnceCell<String> = OnceCell::new();
    USER_AGENT.get_or_init(|| format!("Seedwing/{}", crate::version()))
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("initialization error: {0}")]
    Builder(String),
    #[error("request error: {0}")]
    Request(String),
}

#[derive(Clone, Debug)]
pub struct RemoteClientBuilder {
    client: Result<reqwest::Client, Error>,
}

impl RemoteClientBuilder {
    pub fn new() -> Self {
        let builder = reqwest::ClientBuilder::new();

        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.user_agent(user_agent());

        let client = builder
            .build()
            .map_err(|err| Error::Builder(err.to_string()));

        Self { client }
    }

    pub async fn build(&self) -> Result<RemoteClient, Error> {
        self.client.clone().map(RemoteClient)
    }
}

impl Default for RemoteClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct RemoteClient(reqwest::Client);

pub trait Target {
    fn build_url(self) -> Result<Url, Error>;
}

impl Target for Url {
    fn build_url(self) -> Result<Url, Error> {
        Ok(self)
    }
}

impl Target for (Url, PatternName) {
    fn build_url(self) -> Result<Url, Error> {
        self.0
            .join(&self.1.as_type_str())
            .map_err(|err| Error::Request(err.to_string()))
    }
}

impl RemoteClient {
    /// execute the remote evaluation
    pub async fn evaluate(
        self,
        target: impl Target,
        input: &RuntimeValue,
    ) -> Result<Response, Error> {
        let url = target.build_url()?;

        let response = self
            .0
            .post(url)
            .header(http::header::ACCEPT, "application/json")
            .json(&input.as_json())
            .send()
            .await
            .map_err(|err| Error::Request(err.to_string()))?;

        log::info!("Remote response: {}", response.status());

        let response = match response.status() {
            StatusCode::OK | StatusCode::UNPROCESSABLE_ENTITY => response
                .json()
                .await
                .map_err(|err| Error::Request(err.to_string()))?,
            code => {
                return Err(Error::Request(format!(
                    "Invalid remote response code: {code}"
                )))
            }
        };

        log::info!("Response: {response:#?}");

        Ok(response)
    }
}
