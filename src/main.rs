mod openai;

use tokio::sync::Mutex;

use openai::OpenAiError;
use poise::{
    command,
    serenity_prelude::{self as serenity, Message},
};
use tracing::{error, info, instrument};
use tracing_subscriber::{prelude::*, EnvFilter};

// User data, which is stored and accessible in all command invocations
#[derive(Debug)]
struct Data {
    personality: Mutex<String>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Takes a text prompt and creates a lovely image
#[instrument(skip(ctx))]
#[command(slash_command)]
async fn paint(
    ctx: Context<'_>,
    #[description = "A text description for prompty to work off of"] description: String,
) -> Result<(), Error> {
    info!("Received paint request");
    // It can take some time for openai to respond so send a defferal to discord to give us more time
    ctx.defer().await?;
    info!("Submitting paint request to OpenAI");
    match openai::get_openai_image(&description).await {
        Ok(bytes) => {
            info!("Received valid response from OpenAI");
            let file = (&bytes[..], "ai_response.png");
            ctx.send(|m| {
                m.attachment(file.into())
                    .embed(|e| e.title(&description).attachment("ai_response.png"))
            })
            .await?;
            info!("Posted painting")
        }
        Err(error) => handle_openai_error(ctx, &error).await,
    }
    Ok(())
}

/// Draw an image describing this messages content
#[instrument(skip(ctx))]
#[command(context_menu_command = "Draw Message")]
async fn paint_message(
    ctx: Context<'_>,
    #[description = "A message to draw an image from"] message: Message,
) -> Result<(), Error> {
    info!("Received paint message request");
    ctx.defer_ephemeral().await?;
    let thread = message
        .channel_id
        .create_public_thread(ctx, &message, |f| f.name("Drawing"))
        .await?;
    info!("Submitting paint request to OpenAI");
    match openai::get_openai_image(&message.content).await {
        Ok(bytes) => {
            info!("Received valid response from OpenAI");
            let file = (&bytes[..], "ai_response.png");
            thread
                .send_message(ctx, |m| {
                    m.add_file(file)
                        .embed(|e| e.title(&message.content).attachment("ai_response.png"))
                })
                .await?;
            ctx.say("All done!!!").await?;
            info!("Posted painting")
        }
        Err(error) => handle_openai_error(ctx, &error).await,
    }
    Ok(())
}

/// Ask the bot a question
#[instrument(skip(ctx))]
#[command(slash_command)]
async fn ask(
    ctx: Context<'_>,
    #[description = "A question for the bot to answer"] question: String,
) -> Result<(), Error> {
    info!("Received question");
    ctx.defer().await?;
    info!("Submitting question to OpenAI");
    let personality = ctx.data().personality.lock().await;
    match openai::get_openai_chat(personality.clone(), question.clone()).await {
        Ok(response) => {
            info!("Received valid response from OpenAI");
            ctx.send(|m| m.embed(|e| e.title(&question).description(response)))
                .await?;
            info!("Posted answer")
        }
        Err(error) => handle_openai_error(ctx, &error).await,
    }
    Ok(())
}

/// Ask the bot a question
#[instrument(skip(ctx))]
#[command(
    context_menu_command = "Ask Prompty",
    required_permissions = "SEND_MESSAGES"
)]
async fn ask_context(
    ctx: Context<'_>,
    #[description = "A message to reply to"] message: Message,
) -> Result<(), Error> {
    info!("Received question");
    ctx.defer_ephemeral().await?;
    info!("Submitting question to OpenAI");
    let personality = ctx.data().personality.lock().await;
    match openai::get_openai_chat(personality.clone(), message.content.clone()).await {
        Ok(response) => {
            info!("Received valid response from OpenAI");
            message.reply(ctx, response).await?;
            ctx.say("All Done!!!").await?;
            info!("Posted answer")
        }
        Err(error) => handle_openai_error(ctx, &error).await,
    }
    Ok(())
}

#[command(slash_command, subcommands("set", "get"))]
async fn personality(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// Set the bots personality
#[instrument(skip(ctx))]
#[command(slash_command)]
async fn get(ctx: Context<'_>) -> Result<(), Error> {
    let existing_personality = ctx.data().personality.lock().await;
    ctx.say(format!("My personality is: {existing_personality}",))
        .await?;
    Ok(())
}

// Set the bots personality
#[instrument(skip(ctx))]
#[command(slash_command)]
async fn set(
    ctx: Context<'_>,
    #[description = "Set the bots personality with a description"] new_personality: String,
) -> Result<(), Error> {
    info!("Received personality description");
    let mut existing_personality = ctx.data().personality.lock().await;
    *existing_personality = new_personality;
    ctx.say(format!(
        "Got it! From now on my personality is: {existing_personality}",
    ))
    .await?;
    info!("Set personality");
    Ok(())
}

async fn handle_openai_error(ctx: Context<'_>, error: &OpenAiError) {
    error!("OpenAi request resulted in error: {:?}", &error);
    let result = match error {
        OpenAiError::Safety => ctx.say("Bonk!!! Go directly to horny jail").await,
        OpenAiError::LimitReached => {
            ctx.say("Looks like I'm all out of paint this month :(")
                .await
        }
        _ => {
            ctx.say("Uh oh something went wrong while I was trying to respond!")
                .await
        }
    };
    if let Err(response_error) = result {
        error!(
            "Encountered error while trying to inform user of error: {:?}",
            response_error
        )
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    dotenv::dotenv().ok();

    let _sentry = sentry::init((
        std::env::var("SENTRY_DSN").expect("missing SENTRY_DSN"),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ));

    tracing_subscriber::Registry::default()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(sentry::integrations::tracing::layer())
        .init();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                paint(),
                paint_message(),
                ask(),
                ask_context(),
                personality(),
            ],
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    personality: Mutex::new("I'm a helpful assistant".to_string()),
                })
            })
        });

    framework.run().await.unwrap();
}
