use std::env;
use sorted_insert::SortedInsertByKey;

use std::sync::Arc;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Duration as ChronoDuration;
use chrono::Local;
use chrono::Months;
use chrono::TimeZone;
use serenity::all::CreateMessage;
use serenity::all::UserId;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use dotenv::dotenv;
use chrono::Timelike;

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration as TokioDuration};

// User Lib imports
use date::date_utils::{parse_date, RemindMeDateTypes};

struct Handler {
    tx: Arc<mpsc::Sender<RemindMeJob>>
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
                    future_datetime = future_datetime + ChronoDuration::hours(h.into());
                    future_datetime += ChronoDuration::minutes(min.into());
                    future_datetime += ChronoDuration::hours(h.into());
                    future_datetime += ChronoDuration::days(d.into());
                    future_datetime = future_datetime + Months::new(mon);

                    let new_year = future_datetime.year() + y;

                    // TODO error checking
                    future_datetime = future_datetime.with_year(new_year).unwrap();
                },
            }

            // TODO implement second token (so you can add time with years)
            let mut message_start_index = 2;
            match second_token_type {
                // TODO : This is ripe for DRY refactoring
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
            let job = RemindMeJob { date: future_datetime,
                msg: String::from(&tokens[message_start_index..].join(" ")),
                ctx: ctx,
                user_id: msg.author.id,
                written: false,
             };

            // TODO this will send a date job to the rx later on
            let _ = self.tx.send(job).await;
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

struct RemindMeJob {
    date: DateTime<Local>, // TODO: Check to see if Local is appropriate to use (for user in different time zone)
    msg: String,
    ctx: Context,
    user_id: UserId,
    written: bool
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
    let rw_vec: Arc<RwLock<Vec<RemindMeJob>>> = Arc::new(RwLock::new(Vec::new()));


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
    let (tx, mut rx) = mpsc::channel::<RemindMeJob>(30);
    let ipc_tx = Arc::new(tx);

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler { tx: ipc_tx.clone()}).await.expect("Err creating client");

    let rw_map_clone = rw_vec.clone();
    let remindme_job_thread = tokio::spawn(async move {
        while let Some(job) = rx.recv().await { // Continuously wait for messages
            println!("Received: {}", job.msg);

            let mut write = rw_map_clone.write().await;
            write.sorted_insert_asc_by_key(job, |e| &e.date);
    }
    });

    let rw_vec_clone = rw_vec.clone();
    let readme_job_thread = tokio::spawn(async move {
        loop {
            let mut write = rw_vec_clone.write().await;
            println!("DB!");

            let now = Local::now();
            for entry in write.iter_mut() {
                println!("date: {}, msg: {}, written: {}", entry.date, entry.msg, entry.written);
                if entry.written == false {
                    if entry.date < now {
                        entry.written = true;
                        send_direct_msg_job(&entry.ctx, &entry.user_id, &entry.msg).await;
                    }
                }
            }
            let _ = sleep(TokioDuration::from_secs(10)).await;
        }
    });

    let rw_vec_clone = rw_vec.clone();
    let deleteme_job_thread = tokio::spawn(async move {
        loop {
            let mut write = rw_vec_clone.write().await;
            write.retain(|entry| entry.written == false);
            //TODO this time doesn't need to be low
            let _ = sleep(TokioDuration::from_secs(10)).await;
        }
    });

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }

    let _ = remindme_job_thread.await;
    let _ = readme_job_thread.await;
    let _ = deleteme_job_thread.await;
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