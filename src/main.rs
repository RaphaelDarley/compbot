use std::env;

use compbot::db_utils;
use serenity::all::{Member, Role};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        let user = member.user.clone();
        let username = user.name;
        println!("user: {username}, has joined!");
        let guild_id = member.guild_id;
        let Ok(guild) = guild_id.to_partial_guild(&ctx).await else {
            return;
        };

        let Some(member_role) = guild.role_by_name("Member") else {
            return;
        };
        let Some(non_member_role) = guild.role_by_name("Non-Member") else {
            return;
        };

        if db_utils::check_username(&username).await {
            println!("user: {username}, is a member");
            member.add_role(&ctx, member_role.id).await.ok();
            member.remove_role(&ctx, non_member_role.id).await.ok();
        } else {
            println!("user: {username}, is not a member, further verification required");
            member.add_role(&ctx, non_member_role.id).await.ok();
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
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
