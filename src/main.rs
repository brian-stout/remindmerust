use std::env;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use serenity::all::CreateMessage;
use serenity::all::UserId;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use dotenv::dotenv;
use regex::Regex;
use tokio::runtime::Runtime;
use chrono::{DateTime, Utc, NaiveDateTime};
use chrono::format::ParseError;


struct Handler {
    tx: Arc<mpsc::Sender<(Context, UserId, String)>>
}

#[derive(PartialEq, Eq)]
enum RemindMeDateTypes {
    Invalid,
    ThreeLetterMonth { d: u32, mon: String, y: u32 },
    SpecifiedTime{ h: u32, min: u32 },
    AddedTime{ y: u32, mon: u32, d: u32, h: u32, min: u32 },
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event. This is called whenever a new message is received.
    //
    // Event handlers are dispatched through a threadpool, and so multiple events can be
    // dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            // Sending a message can fail, due to a network error, an authentication error, or lack
            // of permissions to post in the channel, so log to stdout when some error happens,
            // with a description of it.
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
        if msg.content.starts_with("!remindme") {
            println!("The message is: {}", msg.content);
            let tokens = msg.content.split(" ").collect::<Vec<&str>>();
            for token in &tokens {
                println!("{}", token);
            }

            if tokens.len() < 3 {
                channel_reply(&msg, &ctx, r"Not enough information, ex: !remindme 3h [message]").await;
                return;
            }

            let first_token = tokens.get(1).unwrap();
            let first_token_type = parse_date(first_token);
            if first_token_type == RemindMeDateTypes::Invalid {
                channel_reply(&msg, &ctx, format!("First token, {first_token}, is not a valid date or time, ex: !remindme [3h | 1300 | 13APR2026] [message]")).await;
                return;
            }

            let second_token = tokens.get(2).unwrap();
            let second_token_type = parse_date(second_token);

            // Logic for handling 2nd token when command instead of message
            if second_token_type != RemindMeDateTypes::Invalid {
              if tokens.len() < 4 {
                channel_reply(&msg, &ctx, format!("Missing message, ex: !remindme 13APR2026 1300 [message]")).await;
                return;
              }  
              // TODO extrapolate this out to multiple date types to error check this
              if matches!(first_token_type, RemindMeDateTypes::ThreeLetterMonth{..}) && matches!(second_token_type, RemindMeDateTypes::ThreeLetterMonth{..}) {
                channel_reply(&msg, &ctx, format!("More than one date provided")).await;
                return;
              }

              // TODO replace with match and extrapolate out to multiple date types (possibly support !remindme 1300 3d4m?)
              let first_token_is_time = matches!(first_token_type, RemindMeDateTypes::SpecifiedTime{..}) || matches!(first_token_type, RemindMeDateTypes::AddedTime{..});
              let second_token_is_time = matches!(second_token_type,RemindMeDateTypes::SpecifiedTime{..}) || matches!(second_token_type, RemindMeDateTypes::AddedTime{..});
              if first_token_is_time && second_token_is_time {
                channel_reply(&msg, &ctx, format!("More than one time provided provided")).await;
                return;
              }
            }

            // Parse time for message
            let mut year;
            let mut month = String::new();
            let mut day;
            let mut hour;
            let mut minute;

            match first_token_type {
                RemindMeDateTypes::Invalid=>{},
                RemindMeDateTypes::ThreeLetterMonth { d, mon, y } => {
                    year = y;
                    month = mon;
                    day = d;
                },
                RemindMeDateTypes::SpecifiedTime { h, min } => {
                    hour = h;
                    minute = min;
                },
                RemindMeDateTypes::AddedTime { y, mon, d, h, min } => {

                },
            }


            // TODO this will send a date job to the rx later on
            self.tx.send((ctx, msg.author.id, msg.content)).unwrap();
        }   
    }

    // Set a handler to be called on the `ready` event. This is called when a shard is booted, and
    // a READY payload is sent by Discord. This payload contains data like the current user's guild
    // Ids, current user data, private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

async fn send_direct_msg_job(ctx: &Context, user_id: &UserId, msg: &String) {
    println!("Received message: {}", msg);
    let builder = CreateMessage::new().content(msg);
    if let Err(why) = user_id.direct_message(&ctx.http, builder).await {
               println!("Err sending help: {why:?}");
                // let _ = msg.reply(&ctx, "There was an error DMing you help.").await;
    }
}

// TODO: Set up CTRL+C handling
#[tokio::main]
async fn main() {

    dotenv().ok(); // Reads the .env file

    let token = env::var("DISCORD_TOKEN").expect("Expected a token");

    // match token {
    //     Ok(val) => println!("API_KEY: {:?}", val),
    //     Err(e) => println!("Error API_KEY: {}", e),
    // }
    // Configure the client with your Discord bot token in the environment.
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;


    // Create mpsc channels for IPC
    let (tx, rx) = mpsc::channel::<(Context, UserId, String)>();
    let ipc_tx = Arc::new(tx);

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler { tx: ipc_tx.clone()}).await.expect("Err creating client");

    let rx_thread = thread::spawn ( move || {
        loop {
            match rx.recv() {
                Ok((ctx, user_id, msg)) => {
                    println!("Received message: {}", msg);
                        let rt = Runtime::new().unwrap();
                        // TODO: Figure out how tokio is interacting with the rust thread, I understand that the closure is sync in regular rust thread, but I feel like the actual answer is to just pass the rx not in the closure, but this is the recommend way to handle this online
                        rt.block_on(async move {
                            send_direct_msg_job(&ctx, &user_id, &msg).await
                        });
                },
                Err(_) => {
                    println!("RX Thread closing");
                    break;
                },
            }
        }
    });
    
    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }

    rx_thread.join().unwrap();
}

fn parse_date(str: &str) -> RemindMeDateTypes {

    // TODO: Year first date, whatever, do later
    // let year_first = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").unwrap(); // Regex for YYYY-MM-DD format
    // let is_year_first = year_first.is_match(str);

    if str.is_empty() {
        return RemindMeDateTypes::Invalid;
    }

    // Three letter month 
    // TODO factor out regex strings to different date util file
    // TODO don't use group tags, might be faster?
    let three_letter_month_reg = Regex::new(r"^(?<day>\d{1,2})(?<month>\w{3})(?<year>\d{2,4})$").unwrap();
    let three_letter_month_caps = three_letter_month_reg.captures(str).unwrap();
    if three_letter_month_reg.is_match(str) {
        let day = str::parse::<u32>(&three_letter_month_caps["day"]).unwrap_or_default();
        let month = String::from(&three_letter_month_caps["month"]); // TODO should this be &str?
        let year = str::parse::<u32>(&three_letter_month_caps["year"]).unwrap_or_default();
        return RemindMeDateTypes::ThreeLetterMonth{ d: day, mon: month, y: year};
    }

    // Specified time
    let specified_time_reg = Regex::new(r"^(?<hour>([0-2])([0-3]))(?<minute>([0-5])(\d))$").unwrap();
    let specified_time_caps = specified_time_reg.captures(str).unwrap();
    if specified_time_reg.is_match(str) {
        let hour = str::parse::<u32>(&specified_time_caps["hour"]).unwrap_or_default();
        let minute= str::parse::<u32>(&specified_time_caps["minute"]).unwrap_or_default();
        return RemindMeDateTypes::SpecifiedTime{ h: hour, min: minute};
    }

    // Is added time
    let added_time_reg = Regex::new(r"^(?(?<year>\d+)[y])?((?<month>\d+)[M])?((?<day>\d)+[d])?((?<hour>\d)+[h])?((?<minute>\d)+[m])?$").unwrap();
    let added_time_caps = added_time_reg.captures(str).unwrap();
    if added_time_reg.is_match(str) {
        let year = str::parse::<u32>(&added_time_caps["year"]).unwrap_or_default();
        let month = str::parse::<u32>(&added_time_caps["month"]).unwrap_or_default();
        let day = str::parse::<u32>(&added_time_caps["day"]).unwrap_or_default();
        let hour = str::parse::<u32>(&added_time_caps["hour"]).unwrap_or_default();
        let minute= str::parse::<u32>(&added_time_caps["minute"]).unwrap_or_default();
        return RemindMeDateTypes::AddedTime{ y: year, mon: month, d: day, h: hour, min: minute};
    }

    return RemindMeDateTypes::Invalid;
}

async fn channel_reply(msg: &Message, ctx: &Context, str: impl Into<String>) {
    if let Err(why) = msg.channel_id.say(
        &ctx.http,
        str
    ).await {
        println!("Error sending message: {why:?}");
    }
    return;
}