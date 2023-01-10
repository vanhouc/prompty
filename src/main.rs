use bytes::Bytes;
use poise::serenity_prelude::{self as serenity, ChannelId, Message};
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

#[derive(Debug)]
struct Data {} // User data, which is stored and accessible in all command invocations
#[derive(thiserror::Error, Debug)]
enum PaintImageError {
    #[error("OpenAI returned a safety error because the request was inappropriate")]
    SafetyError,
    #[error("The OpenAI account backing the bot reached its spending limit")]
    LimitReachedError,
    #[error("General network error occurred while fetching image")]
    NetworkError,
}

impl From<reqwest::Error> for PaintImageError {
    fn from(_: reqwest::Error) -> Self {
        Self::NetworkError
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Takes a text prompt and creates a lovely image
#[poise::command(slash_command)]
async fn prompt(
    ctx: Context<'_>,
    #[description = "A text prompt for prompty to work off of"] prompt: String,
) -> Result<(), Error> {
    // It can take some time for openai to respond so send a defferal to discord to give us more time
    ctx.defer().await?;
    prompt_internal(ctx, ctx.channel_id(), &prompt).await
}

/// Takes a text prompt and creates a lovely image
#[poise::command(context_menu_command = "Draw Message")]
async fn draw_message(
    ctx: Context<'_>,
    #[description = "A text prompt for prompty to work off of"] message: Message,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let thread = message
        .channel_id
        .create_public_thread(ctx, &message, |f| f.name("Drawing"))
        .await?;
    prompt_internal(ctx, thread.into(), &message.content).await?;
    ctx.say("All done!!!").await?;
    Ok(())
}

async fn prompt_internal(ctx: Context<'_>, channel: ChannelId, prompt: &str) -> Result<(), Error> {
    match get_openai_image(&prompt).await {
        Ok(bytes) => {
            let file = (&bytes[..], "ai_response.png");
            channel
                .send_message(ctx, |m| {
                    m.add_file(file)
                        .embed(|e| e.title(prompt).attachment("ai_response.png"))
                })
                .await?;
        }
        Err(error) => match error {
            PaintImageError::SafetyError => {
                ctx.say("Bonk!!! Go directly to horny jail").await?;
            }
            PaintImageError::LimitReachedError => {
                ctx.say("Looks like I'm all out of paint this month :(")
                    .await?;
            }
            PaintImageError::NetworkError => {
                ctx.say("Uh oh something went wrong while I was painting your reply!")
                    .await?;
            }
        },
    }
    Ok(())
}

async fn get_openai_image(prompt: &str) -> Result<Bytes, PaintImageError> {
    let client = reqwest::Client::new();
    let generation_response = client
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAI_TOKEN").expect("missing OPENAPI_TOKEN"))
        .json(&ImageGenerationRequest { prompt: &prompt })
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
                    return Err(PaintImageError::LimitReachedError);
                }
            }
            if ai_error.error.message.contains("safety") {
                return Err(PaintImageError::SafetyError);
            }
            Err(PaintImageError::NetworkError)
        }
        _ => Err(PaintImageError::NetworkError),
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![prompt(), draw_message()],
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        });

    framework.run().await.unwrap();
}
