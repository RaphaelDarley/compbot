use std::env;

use chrono::Utc;
use compbot::db_utils::{email_lookup, NameType};
use compbot::{
    add_member_role, check_code, commands, is_verified, lookup_name, respond_to_verified,
    send_verif_email, verify_member, VERIF_CODES,
};
use rand::Rng;
use serenity::all::{
    CreateInteractionResponse, CreateInteractionResponseMessage, Interaction, Member,
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        verify_member(&ctx, &member).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => match cmd.data.name.as_str() {
                "verify" => {
                    let Some(member) = &cmd.member else {
                        return;
                    };
                    if is_verified(&ctx, &member).await {
                        return respond_to_verified(&ctx, cmd).await;
                    }

                    let reply = if verify_member(&ctx, &member).await.unwrap() {
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content("Successfully verified: get computering!!!"),
                        )
                    } else {
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(
                                    "Verification didn't work, try using /verify_email_send with the email you used to join CompSoc or contacting the secretary :(",
                                ),
                        )
                    };
                    cmd.create_response(&ctx, reply).await.unwrap();
                }
                "verify_email_send" => {
                    let Some(member) = &cmd.member else {
                        return;
                    };
                    if is_verified(&ctx, &member).await {
                        return respond_to_verified(&ctx, cmd).await;
                    }

                    let email = cmd.data.options.get(0).unwrap().value.as_str().unwrap();

                    println!(
                        "user: {}, is attempting to verify with email {email}",
                        member.user.name
                    );

                    let Some(cs_user) = email_lookup(email).await else {
                        return cmd
                            .create_response(
                                &ctx,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .ephemeral(true)
                                        .content(format!("Can't find email: {email}.\n Try checking your email for 'Your Oxford CompSoc order' or contact the secretary")),
                                ),
                            )
                            .await
                            .unwrap();
                    };

                    let name = format!("{} {}", cs_user.first_name, cs_user.last_name);

                    let code: u64 = {
                        let mut rng = rand::thread_rng();
                        rng.gen_range(0..=999_999)
                    };
                    let res = send_verif_email(email, &name, code).await;
                    // SKIP SENDING FOR TESTING
                    // let res: Result<(), String> = Ok(());

                    let reply = match res {
                        Ok(_u) => {
                            VERIF_CODES
                                .lock()
                                .await
                                .insert((member.user.id, code), Utc::now());
                            println!(
                                "user: {}, code: {code} sent to email: {email}",
                                member.user.name
                            );
                            format!("Email sent to: {email}! Remember to check your spam/junck then use that code with the /verify_email_code command")
                        }
                        Err(e) => {
                            println!(
                                "user: {}, sending to: {email} errored: {e}",
                                member.user.name
                            );
                            "Error sending email, please contact the secretary".to_string()
                        }
                    };

                    cmd.create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(reply),
                        ),
                    )
                    .await
                    .unwrap()
                }
                "verify_email_code" => {
                    let Some(member) = &cmd.member else {
                        return;
                    };
                    if is_verified(&ctx, &member).await {
                        return respond_to_verified(&ctx, cmd).await;
                    }
                    let code = cmd.data.options.get(0).unwrap().value.as_i64().unwrap() as u64;

                    // dump_codes().await;
                    println!(
                        "user: {}, attempting to verify with code: {}",
                        member.user.id, code
                    );
                    let reply = match check_code(&(member.user.id, code)).await {
                        true => {
                            add_member_role(&ctx, &member).await;
                            println!("user: {}, has been verified by email", member.user.name);
                            "You're now verified: get Computering!!!"
                        }
                        false => "Verification failed :( try again or contact the secretary.",
                    };
                    cmd.create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(reply),
                        ),
                    )
                    .await
                    .unwrap();
                }
                "lookup_first" => lookup_name(&ctx, cmd, NameType::First).await,
                "lookup_last" => lookup_name(&ctx, cmd, NameType::Last).await,
                _ => {}
            },
            Interaction::Modal(_) => {}
            _ => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        for guild in ready.guilds {
            guild.id.set_commands(&ctx, commands()).await.unwrap();
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MEMBERS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
