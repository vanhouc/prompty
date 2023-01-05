use poise::serenity_prelude as serenity;
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
    let mut image_response: ImageGenerationResponse = reqwest::Client::new()
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(std::env::var("OPENAPI_TOKEN").expect("missing OPENAPI_TOKEN"))
        .json(&ImageGenerationRequest { prompt: &prompt })
        .send()
        .await?
        .json()
        .await?;
    let response_data = image_response.data.pop().expect("no response was sent");
    // Reply with our generated url
    ctx.send(|m| m.embed(|e| e.title(prompt).image(response_data.url)))
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load environment file");
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
