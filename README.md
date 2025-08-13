# Denedé -- Dice roller bot for Discord
Denedé is a minimalistic Discord bot written in Rust 🦀 for generating dice rolls for D&D games. It uses Rust's [Serenity](https://github.com/serenity-rs/serenity) library. Therefore, it is speedily swift 🔥, slight of weight and mindful of memory!

Denedé supports [MapTool](https://github.com/RPTools/maptool)'s notation for dice rolls. It supports all of MapTool's Dice Expression. For more information, including available operations and expected formats, see [here](https://wiki.rptools.info/index.php/Dice_Expressions).
 
Denedé will read the entirety of the non-bot messages it receives, looking for dice roll patterns, and reply if it finds at least one pattern anywhere in a message. This means that your messages do not have to start with any special character for the bot to trigger. They just have to contain a dice roll in them! For example, the bot will reply to any of the following messages with the requested roll result:
 * "[1d20]"
 * "Charisma check: [1d20+4]"
 * "Flogg takes [2d8+1d6r2] dmg"

For convenience, a `/roll` command is also available, which processes the specified single Dice Expression. The advantage of this method is that it allows for ephemeral responses (i.e., only you will see the result of the roll), which are enabled by default.

The maximum number of rolls the bot will generate for a single dice roll is of 50; and the maximum dice size for any roll is of 1000. Additionally, Denedé tries to mimic the original behavior of MapTool regarding supported operations, formats, and limitations. That includes support for trivial rolls of 0 dice, as well as 1-sided and 0-sided dice, if for any reason you want them.

Also, Denedé uses [RANDOM.ORG](https://www.random.org)'s truly random number generator to resolve the dice rolls. So you can rest assured your rolls are truly random and not pseudo-random!

**Note:** Denedé has a fallback in case RANDOM.ORG's API does not work properly for some reason (e.g.: because it is performing a secure connection / anti-abuse check before serving the random sequence request; it has happened before). In those cases, Denedé will use a pseudo-random number generator from Rust's Random number library instead, to generate the dice rolls. When this occurs, Denedé's response will indicate that the rolls were generated pseudo-randomly by appending `[pseudo-random]` after the roll's result.

