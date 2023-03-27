use bytes::Bytes;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ImageGenerationRequest<'a> {
    prompt: &'a str,
}

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatCompletionMessage>,
}
#[derive(Serialize, Deserialize)]
struct ChatCompletionMessage {
    role: ChatCompletionMessageRole,
    content: String,
}
#[derive(Serialize, Deserialize)]
enum ChatCompletionMessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "assistant")]
    Assistant,
}

// impl Serialize for ChatCompletionMessageRole {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         match &self {
//             ChatCompletionMessageRole::User => serializer.serialize_str("user"),
//             ChatCompletionMessageRole::System => serializer.serialize_str("system"),
//             ChatCompletionMessageRole::Assistant => serializer.serialize_str("assistant"),
//         }
//     }
// }

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionResponseChoice>,
}
#[derive(Deserialize)]
struct ChatCompletionResponseChoice {
    index: u8,
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ImageGenerationResponse {
    data: [ImageGenerationResponseData; 1],
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: OpenAiErrorResponse,
}

#[derive(Deserialize)]
struct OpenAiErrorResponse {
    code: Option<String>,
    message: String,
}

#[derive(Deserialize)]
struct ImageGenerationResponseData {
    url: String,
}

#[derive(thiserror::Error, Debug)]
pub enum OpenAiError {
    #[error("OpenAI returned a safety error because the request was inappropriate")]
    Safety,
    #[error("The OpenAI account backing the bot reached its spending limit")]
    LimitReached,
    #[error("General network error occurred while fetching image")]
    NetworkError,
}

impl From<reqwest::Error> for OpenAiError {
    fn from(_: reqwest::Error) -> Self {
        Self::NetworkError
    }
}

pub async fn get_openai_chat(question: String) -> Result<String, OpenAiError> {
    let client = reqwest::Client::new();
    let chat_response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(std::env::var("OPENAI_TOKEN").expect("missing OPENAPI_TOKEN"))
        .json(
            &(ChatCompletionRequest {
                model: "gpt-3.5-turbo",
                messages: vec![ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: question,
                }],
            }),
        )
        .send()
        .await?;
    match chat_response.status() {
        StatusCode::OK => Ok(chat_response
            .json::<ChatCompletionResponse>()
            .await?
            .choices
            .pop()
            .ok_or(OpenAiError::NetworkError)?
            .message
            .content),
        StatusCode::BAD_REQUEST => {
            let ai_error = chat_response.json::<ErrorResponse>().await?;
            if let Some(code) = ai_error.error.code {
                if code == "billing_hard_limit_reached" {
                    return Err(OpenAiError::LimitReached);
                }
            }
            if ai_error.error.message.contains("safety") {
                return Err(OpenAiError::Safety);
            }
            Err(OpenAiError::NetworkError)
        }
        _ => Err(OpenAiError::NetworkError),
    }
}

pub async fn get_openai_image(prompt: &str) -> Result<Bytes, OpenAiError> {
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
                    return Err(OpenAiError::LimitReached);
                }
            }
            if ai_error.error.message.contains("safety") {
                return Err(OpenAiError::Safety);
            }
            Err(OpenAiError::NetworkError)
        }
        _ => Err(OpenAiError::NetworkError),
    }
}

#[cfg(test)]
mod tests {
    use crate::openai::ChatCompletionResponse;

    #[test]
    fn chat_deserialize() {
        let response = serde_json::from_str::<ChatCompletionResponse>(
            r#"{
            "id": "chatcmpl-6ynZ0ReyzjzMhPjLubNzx1AvPYluQ",
            "object": "chat.completion",
            "created": 1679948222,
            "model": "gpt-3.5-turbo-0301",
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 107,
                "total_tokens": 119
            },
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "As an AI language model, I don't experience emotions. However, love is a complex and multifaceted emotion that can be experienced in many different ways. It can be a profound feeling of affection and care for someone or something, or it can be a deep sense of connection and intimacy with another person. Love can also include feelings of compassion, empathy, and respect, and it can be expressed through actions, words, and behaviors. Ultimately, love is a deeply personal and subjective experience that can mean different things to different people."
                    },
                    "finish_reason": "stop",
                    "index": 0
                }
            ]
        }"#,
        ).unwrap();
        assert_eq!(1, response.choices.len())
    }
}
