use poise::serenity_prelude::{self as serenity};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ImageGenerationRequest<'a> {
    prompt: &'a str,
}

#[derive(Deserialize)]
struct ImageGenerationResponse {
    data: Vec<ImageGenerationResponseData>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: OpenAiError,
}

#[derive(Deserialize)]
struct OpenAiError {
    message: String,
}

#[derive(Deserialize)]
struct ImageGenerationResponseData {
    url: String,
}

#[derive(Debug)]
struct Data {} // User data, which is stored and accessible in all command invocations
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
    let client = reqwest::Client::new();
    let openai_request = client
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAI_TOKEN").expect("missing OPENAPI_TOKEN"))
        .json(&ImageGenerationRequest { prompt: &prompt })
        .send()
        .await;
    match openai_request {
        Ok(response) => match response.status() {
            StatusCode::OK => {
                if let Ok(image_response) = response.json::<ImageGenerationResponse>().await {
                    if let Some(data) = image_response.data.get(0) {
                        if let Ok(image) = client.get(&data.url).send().await {
                            if let Ok(image_bytes) = image.bytes().await {
                                let file = (&image_bytes[..], "ai_response.png");
                                ctx.send(|m| m.content(prompt).attachment(file.into()))
                                    .await?;
                            }
                        }
                    }
                }
            }
            StatusCode::BAD_REQUEST => {
                ctx.say("Bonk!!! Go directly to horny jail").await?;
            }
            _ => (),
        },
        Err(_) => {
            ctx.say("Oh no it appears the artist is unreachable!")
                .await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![prompt()],
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
