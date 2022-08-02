use core::fmt;
use std::fmt::Debug;
use std::io::Error;

use async_trait::async_trait;
use log::{debug, info};
use notion::models::Page as NotionPage;
use notion::NotionApi;
use teloxide::types::{MessageEntity, MessageEntityKind, UpdateKind};
use teloxide::{prelude::*, RequestError};

macro_rules! unpack {
    ($x: expr, $variant:path, $otherwise:expr) => {
        match $x {
            $variant(value) => value,
            _ => $otherwise,
        }
    };
}

fn is_hashtag(e: &&MessageEntity) -> bool {
    if let MessageEntityKind::Hashtag = e.kind {
        true
    } else {
        false
    }
}

trait Matcher {
    fn match_message(&self, m: &Message) -> bool;
}

struct HashTagMatcher {
    hash_tag: String,
}

impl Matcher for HashTagMatcher {
    fn match_message(&self, m: &Message) -> bool {
        let entities = unpack!(m.entities(), Some, return false);
        let text = unpack!(m.text(), Some, return false);
        entities.iter().any(|e| {
            if let MessageEntityKind::Hashtag = e.kind {
                if &text[e.offset..e.offset + e.length] == self.hash_tag {
                    return true;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        })
    }
}

impl fmt::Display for HashTagMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hashtag {} matcher", self.hash_tag)
    }
}

#[async_trait]
trait Destination {
    async fn insert(&self, text: &String);
}

// struct NotionPage {
//     api: &NotionApi,
//     id: String,
// }

#[async_trait]
impl Destination for NotionPage {
    async fn insert(&self, text: &String) {
        info!(
            "properties: {}",
            self.properties.title().unwrap_or("".to_string())
        );
    }
}

enum DestinationType {
    Notion(NotionPage),
}

struct ForwardConfig<T: Matcher> {
    matcher: T,
    destination: DestinationType,
}

fn standup_matcher(text: &&str) -> impl Fn(&&MessageEntity) -> bool {
    let s = String::from(*text);
    // Box::new(move |e: &&MessageEntity| {
    move |e: &&MessageEntity| {
        if &s[e.offset..e.offset + e.length] == "#standup" {
            return true;
        } else {
            return false;
        }
    }
}

fn handle_updates(updates: Vec<Update>) -> () {
    info!("Got {} updates.", updates.len());
    for update in updates {
        info!("Received update id {}: ", update.id);
        let kind = update.kind;
        if let UpdateKind::Message(message) = kind {
            let matcher = HashTagMatcher {
                hash_tag: "#standup".to_string(),
            };
            if matcher.match_message(&message) {
                info!("Found matched message with {}", matcher);
                // store this message to notion
                // first print this whole thing out.
                let text = unpack!(message.text(), Some, return);
                debug!("Matched message text: {}", text)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting agile tg bot...");

    let bot = Bot::from_env();

    let timeout: u32 = 60;

    let mut latest_update: i32 = 0;

    loop {
        let mut req = bot.get_updates();
        req.offset = Some(latest_update);
        req.timeout = Some(timeout);
        info!(
            "Waiting for updates with a timeout of {} seconds and offset of {}",
            timeout, latest_update
        );
        let res = req.send().await;
        match res {
            Err(ref err) => {
                if let RequestError::Network(_) = err {
                    // ignore network error
                    info!("Failed to get update from Telegram because of network error.");
                } else {
                    info!("Failed to get update from Telegram. Error: {}", err);
                }
            }
            Ok(updates) => {
                let len = updates.len();
                if len > 0 {
                    latest_update = match updates
                        .iter()
                        .reduce(|acc, u| if acc.id > u.id { acc } else { u })
                    {
                        Some(update) => update.id + 1,
                        _ => latest_update,
                    };
                    handle_updates(updates);
                }
            }
        }
    }
}
