# Denedé -- Dice roller bot for Discord
Denedé is a minimalistic Discord bot written in Rust 🦀 for generating dice rolls for D&D games. It uses Rust's [Serenity](https://github.com/serenity-rs/serenity) library. Therefore, it is speedily swift 🔥, slight of weight and mindful of memory!

Denedé supports [MapTool](https://github.com/RPTools/maptool)'s notation for dice rolls. Specifically:
 * [NdM] will generate a roll of N M-sided dice. E.g.: [1d20].
 * [NdM+B] will generate a roll of N M-sided dice, and add B as a bonus. E.g.: [1d20+2]. B can be a negative number.
 * [NdM-B] will generate a roll of N M-sided dice, and subtract B as a penalty. E.g.: [1d20-2]. B cannot be a negative number, only positive (no sign specified).
 * Reroll notation is also supported, by adding "r\<R\>" after the roll size, rerolling all results lower than R. E.g.: [1d20r2].
 * There are a also bunch of shortcuts available for rolls. In a roll like [NdMrR], where N, M and R are numbers,
   - If N is omitted, it defaults to 1. E.g.: [d20] is equivalent to [1d20].
   - If M is omitted, it defaults to 20. E.g.: [1d] is equivalent to [1d20].
   - If R is omitted and "r" is not present, it defaults to 1. E.g.: [1d20] is equivalent to [1d20r1].
   - If R is omitted and "r" is present, it defaults to M / 2, rounding up. E.g.: [1d20r] is equivalent to [1d20r10].
   - Multiple of these can be used at the same time. E.g.: [d] is actually equivalent to [1d20r1].

On a technical level, it supports dice rolls that follow one of the following regular expressions for fully-specified rolls without rerolls:
 * `\[\d+d\d+\]` for rolls without an added bonus.
 * `\[\d+d\d+ ?\+ ?-?\d+\]` for rolls with an added bonus (positive or negative).
 * `\[\d+d\d+ ?- ?\d+\]` for rolls with an added negative bonus, a.k.a. penalty.
 
Denedé will read the entirety of the non-bot messages it receives, looking for dice roll patterns, and reply if it finds at least one pattern anywhere in a message. This means that your messages do not have to start with any special character for the bot to trigger. They just have to contain a dice roll in them! For example, the bot will reply to any of the following messages with the requested roll result:
 * "[1d20]"
 * "Flogg takes [2d8+2] dmg"
 * "Charisma check: [1d20+4]"

The maximum number of rolls the bot will generate for a single query is of 20; and the maximum dice size for any roll is of 1000. The maximum bonus supported for a given query is equal to (number of rolls) * (dice size) * 10, to keep everything a reasonable size. It supports trivial rolls of 0 dice, as well as 1-sided and 0-sided dice, if for any reason you want them (although Denedé will note something isn't right about those kinds of rolls).

Also, Denedé uses [RANDOM.ORG](https://www.random.org)'s truly random number generator to resolve the dice rolls. So you can rest assured your rolls are truly random and not pseudo-random!

**Note:** Denedé has a fallback in case RANDOM.ORG's API does not work properly for some reason (e.g.: because it is performing a secure connection / anti-abuse check before serving the random sequence request; it has happened before). In those cases, Denedé will use a pseudo-random number generator from Rust's Random number library instead, to generate the dice rolls. When this occurs, Denedé's response will indicate that the rolls were generated pseudo-randomly by appending `[pseudo-random]` after the roll's result.

