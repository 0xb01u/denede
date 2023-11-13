/*
 *  Denedé: Discord bot for generating D&D dice rolls, written in Rust.
 *  Copyright (C) 2023  Bolu <bolu@tuta.io>
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use std::env;
use regex::Regex;
extern crate reqwest;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::Client;

struct Bot;

#[serenity::async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore messages from other bots:
        if msg.author.bot {
            return;
        }

        let mut content = msg.content;
        let mut response = Vec::new();

        // Shortcut roll message, e.g.: [d] [3d] [d40]
        let dice_shortcut = Regex::new(r"\[d(?<bonus> ?[+-] ?-?\d+)?\]").expect("No shortcut regex?");
        content = dice_shortcut.replace_all(&content, "[1d20$bonus]").into_owned();
        let dice_shortcut_amount = Regex::new(r"\[(?<amount>\d+)d(?<bonus> ?[+-] ?-?\d+)?\]").expect("No amount shortcut regex?");
        content = dice_shortcut_amount.replace_all(&content, "[${amount}d20$bonus]").into_owned();
        let dice_shortcut_size = Regex::new(r"\[d(?<size>\d+)(?<bonus> ?[+-] ?-?\d+)?\]").expect("No size shortcut regex?");
        content = dice_shortcut_size.replace_all(&content, "[1d$size$bonus]").into_owned();

        // Regular roll message, e.g.: [2d20]
        let dice = Regex::new(r"(?<roll>\[\d+d\d+)\]").expect("No un-bonused regex?");
        content = dice.replace_all(&content, "$roll+0]").into_owned();

        // Negative bonus roll message, e.g.: [2d20-5]
        let dice_and_neg_bonus = Regex::new(r"(?<roll>\[\d+d\d+) ?- ?(?<bonus>\d+\])").expect("No negative-bonused regex?");
        content = dice_and_neg_bonus.replace_all(&content, "$roll+-$bonus").into_owned();

        // Bonus roll message, e.g.: [2d20+5]
        let dice_and_bonus = Regex::new(r"\[(\d+)d(\d+) ?\+ ?(-?\d+)\]").expect("No regex?");
        for (_, [rolls_str, size_str, bonus_str]) in dice_and_bonus.captures_iter(&content).map(|c| c.extract()) {
            // Avoid an i64-parse error:
            // (2**63 is 19 characters long.)
            if rolls_str.chars().count() > 18 || size_str.chars().count() > 18 || bonus_str.chars().count() > 18 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content("That numeral is overlarge for mine ancient, fatigued orbs to even peruse. I am apprehensive thou shalt require another's aid. Should thou seek assistance with lesser matters, I am at thy service!")).await;
                continue;
            }

            let rolls = rolls_str.parse::<i64>().expect("No rolls?");
            let size = size_str.parse::<i64>().expect("No size?");
            let bonus = bonus_str.parse::<i64>().expect("No bonus?");
            
            if size > 1 && rolls > 0 {
                // Arbitrary limits check, so only reasonable amounts of numbers of reasonable size are returned:
                if rolls > 20i64 {
                    response.push("Inquired for overmuch rolls. I may only proffer up to twain score!".to_owned());
                    continue;
                }
                if size > 1_000i64 {
                    response.push("Entreaded for an excessive sum. I can only reckon unto a thousand!".to_owned());
                    continue;
                }
                if bonus > rolls * size * 10 {
                    response.push("Besought an excessive boon. Be not so covetous, traveller!".to_owned());
                    continue;
                }

                let url = format!("https://www.random.org/integers/?num={}&min=1&max={}&col=1&base=10&format=plain&rnd=new", rolls, size);
                let res = reqwest::get(url).await.expect("No random?");
                let body = res.text().await.expect("No numbers?");

                // Comma-separated sequence:
                let mut sequence = format!("{}", body.replace("\n", ", "));
                sequence.pop(); // Remove trailing space
                sequence.pop(); // Remove trailing comma

                let sum = sequence.split(", ").map(|n| n.parse::<i64>().unwrap()).reduce(|a, b| a + b).expect("No reduction?");

                if bonus == 0 {
                    if rolls == 1 {
                        response.push(format!("{}", sequence));
                    } else {
                        response.push(format!("{} = {}", sequence, sum + bonus));
                    }
                } else {
                    response.push(format!("{} + {} = {}", sequence, bonus, sum + bonus));
                }
            } else {
                // Smug answer for d1s, d0s, and 0 rolls:
                if rolls > 1_000_000_000 || size > 1_000_000_000 || bonus > 1_000_000_00 {
                   response.push(format!("Deem me not a fool, traveller. Be earnest and cease thy jesting with me!"));
                } else {
                   response.push(format!("I deem thy sagacity to be not especially lofty, thus I shall provide a rejoinder to thy entreaty, as a gesture of courtesy: {}", rolls * size + bonus));
                }
            }
        }
        // Join all rolls in the corresponding amount of messages:
        let mut response_str = "".to_owned();
        for roll in &response {
            if response_str.len() + roll.len() > 2000 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content(response_str)).await;
                response_str = "".to_owned();
            }
            response_str.push_str(&format!("{}\n", roll));
        }
        // Send last response:
        let _ = msg.channel_id.send_message(&ctx, |msg| msg.content(response_str)).await;
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{}#{} is connected.", ready.user.name, ready.user.discriminator);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("No tokens?");
    let mut client = Client::builder(&token, GatewayIntents::default() | GatewayIntents::MESSAGE_CONTENT).event_handler(Bot).await.expect("No clients?");

    client.start().await.expect("No work?");
}

