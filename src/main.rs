mod openai;

use openai::PaintImageError;
use poise::serenity_prelude::{self as serenity, ChannelId, Message};

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
    prompt_internal(ctx, ctx.channel_id(), &prompt).await
}

/// Draw an image describing this messages content
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
    match openai::get_openai_image(prompt).await {
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
            PaintImageError::Safety => {
                ctx.say("Bonk!!! Go directly to horny jail").await?;
            }
            PaintImageError::LimitReached => {
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
