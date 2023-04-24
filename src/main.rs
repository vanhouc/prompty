mod openai;

use std::time::Duration;

use openai::OpenAiError;
use poise::serenity_prelude::{self as serenity, ChannelId, Message};
use tokio::time::sleep;
use tracing::{info, instrument};
use tracing_subscriber::{filter, prelude::*};

#[derive(Debug)]
struct Data {} // User data, which is stored and accessible in all command invocations

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Takes a text prompt and creates a lovely image
#[instrument]
#[poise::command(slash_command)]
async fn paint(
    ctx: Context<'_>,
    #[description = "A text description for prompty to work off of"] description: String,
) -> Result<(), Error> {
    // It can take some time for openai to respond so send a defferal to discord to give us more time
    ctx.defer().await?;
    paint_internal(ctx, None, &description).await
}

/// Draw an image describing this messages content
#[instrument]
#[poise::command(context_menu_command = "Draw Message")]
async fn paint_message(
    ctx: Context<'_>,
    #[description = "A message to draw an image from"] message: Message,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let thread = message
        .channel_id
        .create_public_thread(ctx, &message, |f| f.name("Drawing"))
        .await?;
    paint_internal(ctx, Some(thread.into()), &message.content).await?;
    ctx.say("All done!!!").await?;
    Ok(())
}

/// Ask the bot a question
#[instrument(skip(ctx))]
#[poise::command(slash_command)]
async fn ask(
    ctx: Context<'_>,
    #[description = "A question for the bot to answer"] question: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    match openai::get_openai_chat(question.clone()).await {
        Ok(response) => {
            ctx.send(|m| m.embed(|e| e.title(&question).description(response)))
                .await?;
            return Ok(());
        }
        Err(error) => {
            sentry::capture_error(&error);
            match error {
                OpenAiError::Safety => {
                    ctx.say("Bonk!!! Go directly to horny jail").await?;
                }
                OpenAiError::LimitReached => {
                    ctx.say("Looks like I'm all out of paint this month :(")
                        .await?;
                }
                OpenAiError::NetworkError => {
                    ctx.say("Uh oh something went wrong while I was trying to respond!")
                        .await?;
                }
            }
        }
    }
    Ok(())
}

async fn paint_internal(
    ctx: Context<'_>,
    channel: Option<ChannelId>,
    prompt: &str,
) -> Result<(), Error> {
    match openai::get_openai_image(prompt).await {
        Ok(bytes) => {
            let file = (&bytes[..], "ai_response.png");
            if let Some(channel) = channel {
                channel
                    .send_message(ctx, |m| {
                        m.add_file(file)
                            .embed(|e| e.title(prompt).attachment("ai_response.png"))
                    })
                    .await?;
            } else {
                ctx.send(|m| {
                    m.attachment(file.into())
                        .embed(|e| e.title(prompt).attachment("ai_response.png"))
                })
                .await?;
            }
        }
        Err(error) => {
            sentry::capture_error(&error);
            match error {
                OpenAiError::Safety => {
                    ctx.say("Bonk!!! Go directly to horny jail").await?;
                }
                OpenAiError::LimitReached => {
                    ctx.say("Looks like I'm all out of paint this month :(")
                        .await?;
                }
                OpenAiError::NetworkError => {
                    ctx.say("Uh oh something went wrong while I was trying to respond!")
                        .await?;
                }
            }
        }
    }
    Ok(())
}
#[instrument]
async fn test_func() {
    sleep(Duration::from_secs(1)).await;
    info!("Done Sleeping");
}

#[tokio::main]
#[instrument]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::INFO))
        .with(sentry::integrations::tracing::layer())
        .init();

    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").expect("missing SENTRY_DSN"),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ));

    test_func().await;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![paint(), paint_message(), ask()],
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
