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

        // Regular roll message, e.g.: [2d20]
        let dice = Regex::new(r"\[(\d+)d(\d+)\]").expect("No regex?");
        for (_, [rolls_str, size_str]) in dice.captures_iter(&msg.content).map(|c| c.extract()) {
            let rolls = rolls_str.parse::<u32>().expect("No rolls?");
            let size = size_str.parse::<u32>().expect("No size?");
            
            // Arbitrary limits check, so only reasonable amounts of numbers of reasonable size are returned;
            if rolls > 20u32 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content("Inquired for overmuch rolls. I may only proffer up to twain score!")).await;
                continue;
            }
            if size > 1_000u32 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content("Entreaded for an excessive sum. I can only reckon unto a thousand!")).await;
                continue;
            }

            let url = format!("https://www.random.org/integers/?num={}&min=1&max={}&col=1&base=10&format=plain&rnd=new", rolls, size);
            let res = reqwest::get(url).await.expect("No random?");
            let body = res.text().await.expect("No numbers?");

            let mut sequence = format!("{}", body.replace("\n", ", "));
            sequence.pop(); // Remove trailing space
            sequence.pop(); // Remove trailing comma

            if rolls > 1 {
                let sum = sequence.split(", ").map(|n| n.parse::<u32>().unwrap()).reduce(|a, b| a + b).expect("No reduction?");

                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content(format!("{} = {}", sequence, sum))).await;
            } else {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content(format!("{}", sequence))).await;
            }
        }

        // FIXME: As it is right now, first all non-bonus roll will be processed, then all bonus rolls.
        // Fix so that all rolls are processed in order, regardless of bonus.
        // (I.e.: use a regex that captures an optional group for the bonus.)

        // Bonus roll message, e.g.: [2d20+5]
        let dice_and_bonus = Regex::new(r"\[(\d+)d(\d+) ?\+ ?(\d+)\]").expect("No regex?");
        for (_, [rolls_str, size_str, bonus_str]) in dice_and_bonus.captures_iter(&msg.content).map(|c| c.extract()) {
            let rolls = rolls_str.parse::<u32>().expect("No rolls?");
            let size = size_str.parse::<u32>().expect("No size?");
            let bonus = bonus_str.parse::<u32>().expect("No bonus?");
            
            // Arbitrary limits check, so only reasonable amounts of numbers of reasonable size are returned;
            if rolls > 20u32 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content("Inquired for overmuch rolls. I may only proffer up to twain score!")).await;
                continue;
            }
            if size > 1_000u32 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content("Entreaded for an excessive sum. I can only reckon unto a thousand!")).await;
                continue;
            }
            if bonus > rolls * size * 10 {
                let _ = msg.channel_id.send_message(&ctx, |msg| msg.content("Besought an excessive boon. Be not so covetous, traveler!")).await;
                continue;
            }

            let url = format!("https://www.random.org/integers/?num={}&min=1&max={}&col=1&base=10&format=plain&rnd=new", rolls, size);
            let res = reqwest::get(url).await.expect("No random?");
            let body = res.text().await.expect("No numbers?");

            let mut sequence = format!("{}", body.replace("\n", ", "));
            sequence.pop(); // Remove trailing space
            sequence.pop(); // Remove trailing comma

            let sum = sequence.split(", ").map(|n| n.parse::<u32>().unwrap()).reduce(|a, b| a + b).expect("No reduction?");

            let _ = msg.channel_id.send_message(&ctx, |msg| msg.content(format!("{} + {} = {}", sequence, bonus, sum + bonus))).await;
        }
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

