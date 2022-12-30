use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ImageGenerationRequest {
    prompt: String,
}

#[derive(Deserialize)]
struct ImageGenerationResponse {
    data: Vec<ImageGenerationResponseData>,
}

#[derive(Deserialize)]
struct ImageGenerationResponseData {
    url: String,
}

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

    // Send the prompt to openai and get our result image url
    let image_response: ImageGenerationResponse = reqwest::Client::new()
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAPI_TOKEN").expect("missing OPENAPI_TOKEN"))
        .json(&ImageGenerationRequest {
            prompt: prompt.clone(),
        })
        .send()
        .await?
        .json()
        .await?;

    // Reply with our generated url
    ctx.send(|m| m.embed(|e| e.title(prompt).image(image_response.data[0].url.to_owned())))
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
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
