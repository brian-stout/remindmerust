use std::env;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use chrono::Datelike;
use chrono::Duration;
use chrono::Local;
use chrono::Months;
use chrono::TimeZone;
use regex::Captures;
use serenity::all::CreateMessage;
use serenity::all::UserId;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use dotenv::dotenv;
use regex::Regex;
use tokio::runtime::Runtime;
use chrono::Timelike;

struct Handler {
    tx: Arc<mpsc::Sender<(Context, UserId, String)>>
}

#[derive(PartialEq, Eq)]
enum RemindMeDateTypes {
    Invalid,
    ThreeLetterMonth { d: u32, mon: u32, y: i32 },
    SpecifiedTime{ h: u32, min: u32 },
    AddedTime{ y: i32, mon: u32, d: u32, h: u32, min: u32 },
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
            let tokens = msg.content.split(" ").collect::<Vec<&str>>();

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
            let now_datetime = Local::now();
            let mut future_datetime = now_datetime.clone();

            println!("Current time: {now_datetime}");

            match first_token_type {
                RemindMeDateTypes::Invalid => { unreachable!(); }
                RemindMeDateTypes::ThreeLetterMonth { d, mon, y } => {
                    let modified_time = chrono::Local.with_ymd_and_hms(y, mon, d, now_datetime.hour(), now_datetime.minute(), now_datetime.second()).unwrap();

                    if modified_time < now_datetime {
                        channel_reply(&msg, &ctx, format!("Specified time is not in the future")).await;
                        return;
                    }
                    future_datetime = modified_time;
                },
                RemindMeDateTypes::SpecifiedTime { h, min } => {
                    let specified_time = future_datetime.clone();

                    let month = specified_time.month();
                    let day = specified_time.day();
                    let modified_time = chrono::Local.with_ymd_and_hms(specified_time.year(), month, day, h, min, 0).unwrap();

                    if modified_time < now_datetime {
                        channel_reply(&msg, &ctx, format!("Specified time is not in the future")).await;
                        return;
                    }
                    future_datetime = modified_time;
                },
                RemindMeDateTypes::AddedTime { y, mon, d, h, min } => {
                    // TODO eliminate case where first command token is date and  then second command  token is added time?
                    //  or first token is added  time then Second token is a date? Should be eliminated just to simplify the program
                    future_datetime = future_datetime + Duration::hours(h.into());
                    future_datetime += Duration::minutes(min.into());
                    future_datetime += Duration::hours(h.into());
                    future_datetime += Duration::days(d.into());
                    future_datetime = future_datetime + Months::new(mon);

                    let new_year = future_datetime.year() + y;

                    // TODO error checking
                    future_datetime = future_datetime.with_year(new_year).unwrap();
                },
            }

            // TODO implement second token (so you can add time with years)
            let mut message_start_index = 2;
            match second_token_type {
                RemindMeDateTypes::SpecifiedTime { h, min } => {
                    let specified_time = future_datetime.clone();

                    let month = specified_time.month();
                    let day = specified_time.day();
                    let modified_time = chrono::Local.with_ymd_and_hms(specified_time.year(), month, day, h, min, 0).unwrap();

                    if modified_time < now_datetime {
                        channel_reply(&msg, &ctx, format!("Specified time is not in the future")).await;
                        return;
                    }

                    message_start_index = 3;
                    future_datetime = modified_time;
                },
                _ => {},
                // RemindMeDateTypes::AddedTime { y, mon, d, h, min } => todo!(),
            }

            println!("Future datetime: {future_datetime}");
            println!("The message: {}", &tokens[message_start_index..].join(" "));
            channel_reply(&msg, &ctx, format!("You got it! (Actually not implemented)")).await;

            // TODO this will send a date job to the rx later on
            // self.tx.send((ctx, msg.author.id, msg.content)).unwrap();
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
    // TODO: Refactor function to return result
    if str.is_empty() {
        return RemindMeDateTypes::Invalid;
    }

    // Three letter month 
    // TODO factor out regex strings to different date util file
    // TODO don't use group tags, might be faster?
    // TODO the captures unwrap shouldn't happen inside is_match but outside it will, should check anyway
    // TODO the regex should be unit tested
    let three_letter_month_reg = Regex::new(r"^(?<day>\d{1,2})(?<month>\w{3})(?<year>\d{2,4})$").unwrap();
    if three_letter_month_reg.is_match(str) {
        let three_letter_month_caps = three_letter_month_reg.captures(str).unwrap();

        let day = str::parse::<u32>(&get_cap_or_empty_string(&three_letter_month_caps, "day")).unwrap_or_default();
        let month = get_cap_or_empty_string(&three_letter_month_caps, "month");

        let month_num = month_to_number(&month);
        if month_num.is_none() { return RemindMeDateTypes::Invalid }

        let year = str::parse::<i32>(&get_cap_or_empty_string(&three_letter_month_caps, "year")).unwrap_or_default();


        return RemindMeDateTypes::ThreeLetterMonth{ d: day, mon: month_num.unwrap(), y: year};
    }

    // Specified time
    let specified_time_reg = Regex::new(r"^(?<hour>([0-2])([0-3]))(?<minute>([0-5])(\d))$").unwrap();
    if specified_time_reg.is_match(str) {
        let specified_time_caps = specified_time_reg.captures(str).unwrap();

        let hour = str::parse::<u32>(&get_cap_or_empty_string(&specified_time_caps, "hour")).unwrap_or_default();
        let minute = str::parse::<u32>(&get_cap_or_empty_string(&specified_time_caps, "minute")).unwrap_or_default();
        return RemindMeDateTypes::SpecifiedTime{ h: hour, min: minute};
    }

    // Is added time
    // TODO regex definitely needs unit testing, thanks god for the debugger and vim %
    let added_time_reg = Regex::new(r"^((?<year>\d+)[y])?((?<month>\d+)[M])?((?<day>\d+)[d])?((?<hour>\d+)[h])?((?<minute>\d+)[m])?$").unwrap();
    if added_time_reg.is_match(str) {
        let added_time_caps = added_time_reg.captures(str).unwrap();
        let year = str::parse::<i32>(&get_cap_or_empty_string(&added_time_caps, "year")).unwrap_or_default();
        let month = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "month")).unwrap_or_default();
        let day = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "day")).unwrap_or_default();
        let hour = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "hour")).unwrap_or_default();
        let minute = str::parse::<u32>(&get_cap_or_empty_string(&added_time_caps, "minute")).unwrap_or_default();

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

fn month_to_number(mon: &str) -> Option<u32> {
    match mon.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

// TODO check to see if we can fix the lifetime error and return string ref
fn get_cap_or_empty_string(caps: &Captures<'_>, name: &str) -> String {
    return String::from(caps.name(name).map_or("", |m| m.as_str()));
}