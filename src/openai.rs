use bytes::Bytes;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ImageGenerationRequest<'a> {
    prompt: &'a str,
}

#[derive(Deserialize)]
struct ImageGenerationResponse {
    data: [ImageGenerationResponseData; 1],
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: OpenAiError,
}

#[derive(Deserialize)]
struct OpenAiError {
    code: Option<String>,
    message: String,
}

#[derive(Deserialize)]
struct ImageGenerationResponseData {
    url: String,
}

#[derive(thiserror::Error, Debug)]
pub enum PaintImageError {
    #[error("OpenAI returned a safety error because the request was inappropriate")]
    Safety,
    #[error("The OpenAI account backing the bot reached its spending limit")]
    LimitReached,
    #[error("General network error occurred while fetching image")]
    NetworkError,
}

impl From<reqwest::Error> for PaintImageError {
    fn from(_: reqwest::Error) -> Self {
        Self::NetworkError
    }
}

pub async fn get_openai_image(prompt: &str) -> Result<Bytes, PaintImageError> {
    let client = reqwest::Client::new();
    let generation_response = client
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAI_TOKEN").expect("missing OPENAPI_TOKEN"))
        .json(&ImageGenerationRequest { prompt })
        .send()
        .await?;
    match generation_response.status() {
        StatusCode::OK => client
            .get(
                &generation_response
                    .json::<ImageGenerationResponse>()
                    .await?
                    .data[0]
                    .url,
            )
            .send()
            .await?
            .bytes()
            .await
            .map_err(|e| e.into()),
        StatusCode::BAD_REQUEST => {
            let ai_error = generation_response.json::<ErrorResponse>().await?;
            if let Some(code) = ai_error.error.code {
                if code == "billing_hard_limit_reached" {
                    return Err(PaintImageError::LimitReached);
                }
            }
            if ai_error.error.message.contains("safety") {
                return Err(PaintImageError::Safety);
            }
            Err(PaintImageError::NetworkError)
        }
        _ => Err(PaintImageError::NetworkError),
    }
}
