# Denedé -- Dice roller bot for Discord
Denedé is a minimalistic Discord bot written in Rust 🦀 for generating dice rolls for D&D games. It uses Rust's [Serenity](https://github.com/serenity-rs/serenity) library. Therefore, it is speedily swift 🔥, slight of weight and mindful of memory!

Denedé supports [MapTool](https://github.com/RPTools/maptool)'s notation for dice rolls. Specifically:
 * [NdM] will generate N rolls of an M-sized dice. E.g.: [1d20].
 * [NdM+B] will generate N rolls of an M-sized dice, and add B as a bonus. E.g.: [1d20+2].

On a technical level, it supports dice rolls that follow one of the following regular expressions:
 * r"\[(\d+)d(\d+)\]" for rolls without an added bonus.
 * r"\[(\d+)d(\d+) ?\+ ?(\d+)\]" for rolls with an added bonus
 
Denedé will read the entirety of the messages it receives, looking for dice roll patterns, and replying if it finds at least one of them anywhere in the message. This means that your messages do not have to start with any special character for the bot to trigger. They just have to contain a dice roll in them! For example, the bot will reply to any of the following messages with the requested roll result:
 * "[1d20]"
 * "Flogg takes [2d8+2] dmg"
 * "Charisma check: [1d20+4]"

The maximum number of rolls the bot will generate for a single query is of 20; and the maximum dice size for any roll is of 1000. The maximum bonus supported for a given query is equal to (number of rolls) * (dice size) * 10, to keep everything a reasonable size.

Also, Denedé uses [RANDOM.org](https://www.random.org)'s truly random number generator to resolve the dice rolls. So you can rest assured your rolls are truly random and not pseudo-random!

