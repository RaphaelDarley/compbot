use chrono::{DateTime, TimeDelta, Utc};
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, Member, UserId,
};
use tokio::sync::Mutex;

pub mod db_utils;

pub static VERIF_CODES: Lazy<Mutex<HashMap<(UserId, u64), DateTime<Utc>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
pub const TIMEOUT_MINS: i64 = 30;
pub async fn dump_codes() {
    let codes = VERIF_CODES.lock().await;
    println!("code dump:");
    for (k, v) in codes.iter() {
        println!("user: {}, code: {}, at {}", k.0, k.1, v.to_rfc3339())
    }
    println!("dump ended----------------");
}

fn expired(ts: &DateTime<Utc>) -> bool {
    ts.signed_duration_since(Utc::now()) > TimeDelta::try_minutes(TIMEOUT_MINS).unwrap()
}

pub async fn clean_codes() {
    let mut codes = VERIF_CODES.lock().await;
    let to_remove: Vec<(UserId, u64)> = codes
        .iter()
        .filter(|(_, ts)| expired(ts))
        .map(|(k, _)| k)
        .cloned()
        .collect();
    for k in to_remove {
        codes.remove(&k);
    }
}

pub async fn check_code(k: &(UserId, u64)) -> bool {
    let mut codes = VERIF_CODES.lock().await;
    match codes.get(k) {
        Some(ts) => {
            if expired(ts) {
                clean_codes().await;
                false
            } else {
                codes.remove(k);
                true
            }
        }
        None => false,
    }
}

pub struct CSUser {
    pub id: u64,
    pub first_name: String,
    pub last_name: String,
}

impl From<(u64, String, String)> for CSUser {
    fn from(value: (u64, String, String)) -> Self {
        CSUser {
            id: value.0,
            first_name: value.1,
            last_name: value.2,
        }
    }
}

pub fn commands() -> Vec<CreateCommand> {
    let verify = CreateCommand::new("verify").description("verify to get member role");
    let verify_email_send = CreateCommand::new("verify_email_send")
        .description("send verification email")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "email",
                "the email you used to joing CompSoc",
            )
            .required(true),
        );
    let verify_email_code = CreateCommand::new("verify_email_code")
        .description("enter code sent to your email to verify")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "code",
                "the email you used to joing CompSoc",
            )
            .required(true),
        );

    vec![verify, verify_email_send, verify_email_code]
}

pub async fn add_member_role(ctx: &Context, member: &Member) {
    let guild_id = member.guild_id;
    let guild = guild_id.to_partial_guild(&ctx).await.unwrap();
    let member_role = guild.role_by_name("Member").unwrap();
    let non_member_role = guild.role_by_name("Non-Member").unwrap();
    member.add_role(&ctx, member_role.id).await.unwrap();
    member.remove_role(&ctx, non_member_role.id).await.unwrap();
}

pub async fn add_nonmember_role(ctx: &Context, member: &Member) {
    let guild_id = member.guild_id;
    let guild = guild_id.to_partial_guild(&ctx).await.unwrap();
    let non_member_role = guild.role_by_name("Non-Member").unwrap();
    member.add_role(&ctx, non_member_role.id).await.ok();
}

pub async fn verify_member(ctx: &Context, member: &Member) -> Option<bool> {
    let user = member.user.clone();
    let username = user.name;
    println!("user: {username}, is being checking in the database");

    if db_utils::check_username(&username).await {
        println!("user: {username}, is a member");
        add_member_role(&ctx, &member).await;
        Some(true)
    } else {
        println!("user: {username}, is not a member, further verification required");
        add_nonmember_role(&ctx, &member).await;
        Some(false)
    }
}

pub async fn is_verified(ctx: &Context, member: &Member) -> bool {
    let guild_id = member.guild_id;
    let guild = guild_id.to_partial_guild(&ctx).await.unwrap();
    let member_role = guild.role_by_name("Member").unwrap();
    member.roles.contains(&member_role.id)
}

pub async fn respond_to_verified(ctx: &Context, cmd: CommandInteraction) {
    cmd.create_response(
        &ctx,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("You're already verified!!!"),
        ),
    )
    .await
    .ok();
}

pub async fn send_verif_email(email: &str, name: &str, code: u64) -> Result<(), String> {
    let sg_key = std::env::var("SENDGRID_API_KEY").expect("need SENDGRID_API_KEY to send emails");
    use sendgrid::{Mail, SGClient};

    let message = format!("Hello {name}!\nHere is your CompSoc discord verification code: {code:06}. Use this with the /verify_email_code command.");

    let mail = Mail::new()
        .add_from("secretary@ox.compsoc.net")
        .add_text(&message)
        .add_subject("CompSoc verification code")
        .add_to((email, name).into());
    match SGClient::new(sg_key).send(mail).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
