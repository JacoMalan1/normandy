use std::str::FromStr;

use http::Method;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    requests: Vec<RequestSpec>,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid request path.")]
    Path,
    #[error("Malformed HTTP header.")]
    Header,
    #[error("{0}")]
    HeaderName(#[from] http::header::InvalidHeaderName),
    #[error("{0}")]
    HeaderValue(#[from] http::header::InvalidHeaderValue),
}

impl Config {
    pub fn validate(self) -> Result<Validated, ValidationError> {
        let mut result = Vec::with_capacity(self.requests.len());
        for r in self.requests {
            result.push(r.validate()?);
        }
        Ok(Validated { requests: result })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestSpec {
    method: RequestMethod,
    path: String,
    body: Option<RequestBody>,
    #[serde(default = "Vec::new")]
    headers: Vec<String>,
}

impl RequestSpec {
    pub fn validate(self) -> Result<ValidatedRequest, ValidationError> {
        Ok(ValidatedRequest {
            method: self.method.into(),
            path: self.path.parse().map_err(|_| ValidationError::Path)?,
            body: self.body.map(|body| Vec::from(body.as_bytes())),
            headers: self
                .headers
                .into_iter()
                .flat_map(
                    |x| -> Result<
                        (http::header::HeaderName, http::header::HeaderValue),
                        ValidationError,
                    > {
                        let (name, value) = x.split_once(':').ok_or(ValidationError::Header)?;
                        Ok((
                            http::header::HeaderName::from_str(name.trim())?,
                            http::header::HeaderValue::from_str(value.trim())?,
                        ))
                    },
                )
                .collect(),
        })
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, Deserialize)]
pub enum RequestMethod {
    GET,
    POST,
    PUT,
    PATCH,
    HEAD,
    OPTIONS,
    DELETE,
}

impl From<RequestMethod> for http::method::Method {
    fn from(value: RequestMethod) -> Self {
        match value {
            RequestMethod::GET => http::method::Method::GET,
            RequestMethod::POST => http::method::Method::POST,
            RequestMethod::PUT => http::method::Method::PUT,
            RequestMethod::PATCH => http::method::Method::PATCH,
            RequestMethod::HEAD => http::method::Method::HEAD,
            RequestMethod::OPTIONS => http::method::Method::OPTIONS,
            RequestMethod::DELETE => http::method::Method::DELETE,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum RequestBody {
    Json(String),
}

impl RequestBody {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Json(s) => s.as_bytes(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validated {
    requests: Vec<ValidatedRequest>,
}

impl Validated {
    pub fn requests(&self) -> &[ValidatedRequest] {
        &self.requests
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedRequest {
    method: http::method::Method,
    path: http::uri::PathAndQuery,
    body: Option<Vec<u8>>,
    headers: http::header::HeaderMap,
}

impl ValidatedRequest {
    pub async fn send(self, base_url: &reqwest::Url) -> Result<reqwest::Response, reqwest::Error> {
        match self.method {
            Method::GET => {
                let new_path = format!("{}{}", base_url.path(), self.path);
                let mut new_url = base_url.clone();
                new_url.set_path(&new_path);
                reqwest::get(new_url).await
            }
            Method::POST => {
                let new_path = format!("{}{}", base_url.path(), self.path);
                let mut new_url = base_url.clone();
                new_url.set_path(&new_path);
                let client = reqwest::Client::new();
                let mut builder = client.post(new_url).headers(self.headers);
                if let Some(body) = self.body {
                    builder = builder.body(body);
                }
                builder.send().await
            }
            _ => unimplemented!(),
        }
    }
}
