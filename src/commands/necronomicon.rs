/*
 *  Denedé: Discord bot for generating D&D dice rolls, written in Rust.
 *  Copyright (C) 2023-2024  Bolu <bolu@tuta.io>
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
use serde::Serialize;
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandOptionType, ResolvedOption, ResolvedValue};
use std::env;

/* Data structures (serializable): */
#[derive(Serialize)]
struct EnemyBasicsForm {
    enemy_type: String,
    hp: i16,
    ac: u8,
    mov: u8,
    traits: Vec<String>,
}

#[derive(Serialize)]
struct EnemyAttributesForm {
    str: u8,
    dex: u8,
    con: u8,
    int: u8,
    wis: u8,
    cha: u8,
    str_sav: u8,
    dex_sav: u8,
    con_sav: u8,
    int_sav: u8,
    wis_sav: u8,
    cha_sav: u8,
}

#[derive(Serialize)]
struct EnemyRIVForm {
    resistances: Vec<String>,
    immunities: Vec<String>,
    vulnerabilities: Vec<String>,
}

#[derive(Serialize)]
struct EnemyAbilityForm {
    tree: String,
    name: String,
    description: String,
}

#[derive(Serialize)]
struct RivEffect {
    name: String,
    category: String,
}

#[derive(Serialize)]
struct Trait {
    category: String,
    subcategory: String,
    name: String,
    description: String,
}

/* Macros: */

/**
 * Macro to conveniently access one of the options passed to the command (i.e. arguments).
 * The accessed option may not exist (i.e. can be not required).
 */
#[macro_export]
macro_rules! get_cmd_opt {
    /* Get an option that exists from the list of passed options. */
    ($options:ident, $idx:expr, $type:ident) => {{
        let ResolvedOption {
            value: ResolvedValue::$type(tmp),
            ..
        } = $options.get($idx).expect(
            format!(
                "Could not get option in {}[{}].",
                stringify!($options),
                $idx
            )
            .as_str(),
        )
        else {
            panic!(concat!(
                stringify!($options),
                "[",
                stringify!($idx),
                "] is not of ",
                stringify!($type),
                " type."
            ));
        };
        *tmp
    }};
    /* Get an option that may exist from the tail of the list of passed options. */
    ($options:ident, last, $type:ident, $default:expr) => {{
        if let Some(ResolvedOption {
            value: ResolvedValue::$type(tmp),
            ..
        }) = $options.last()
        {
            *tmp
        } else {
            $default
        }
    }};
    /* Get an option that may exist from the list of passed options. */
    ($options:ident, $idx:expr, $type:ident, $default:expr) => {{
        if let Some(ResolvedOption {
            value: ResolvedValue::$type(tmp),
            ..
        }) = $options.get($idx)
        {
            *tmp
        } else {
            $default
        }
    }};
}

/**
 * Macro to conveniently generate the URL for one of the server's endpoints.
 */
#[macro_export]
macro_rules! endpoint {
    ($end:expr) => {
        format!(
            "{}/{}",
            env::var("SERVER_INTERNAL_URL")
                .expect("SERVER_INTERNAL_URL environmental variable not set."),
            $end,
        )
    };
}

/**
 * Macro to conveniently send petitions to the server and handle all related logic.
 */
#[macro_export]
macro_rules! petition {
    ($req_type:ident, $endpoint:expr, $data:ident, $ephemeral:expr) => {{
        let client = reqwest::Client::new();
        match client
            .$req_type(endpoint!($endpoint))
            .json(&$data)
            .send()
            .await
        {
            Ok(r) => r,
            Err(why) => {
                return Some((
                    format!("Could not send the request to the server: {}.", why),
                    $ephemeral,
                ))
            }
        }
    }};
    ($req_type:ident, $endpoint:expr, $data:ident) => {{
        let client = reqwest::Client::new();
        match client
            .$req_type(endpoint!($endpoint))
            .json(&$data)
            .send()
            .await
        {
            Ok(r) => r,
            Err(why) => {
                return Some((
                    format!("Could not send the request to the server: {}.", why),
                    false,
                ))
            }
        }
    }};
    ($req_type:ident, $endpoint:expr) => {{
        let client = reqwest::Client::new();
        match client.$req_type(endpoint!($endpoint)).send().await {
            Ok(r) => r,
            Err(why) => {
                return Some((
                    format!("Could not send the request to the server: {}.", why),
                    false,
                ))
            }
        }
    }};
}

/**
 * Macro for the remaining/unexpected/default match cases, when a server response with an
 * unexpected status code is received.
 */
#[macro_export]
macro_rules! unexpected_response {
    ($response:ident, $ephemeral:expr) => {
        Some((
            format!(
                "Something went wrong: Response code {}.",
                $response.status().as_str()
            ),
            $ephemeral,
        ))
    };
    ($response:ident) => {
        Some((
            format!(
                "Something went wrong: Response code {}.",
                $response.status().as_str()
            ),
            false,
        ))
    };
}

/* Utility functions: */
fn sanitize_name(name: &str) -> String {
    name.to_lowercase().replace(" ", "_")
}

/* Command functions: */

const NOT_FOUND_MSG: &str = "Could not find the specified enemy on the system.
(Or something very wrong happened to the server.)";

pub async fn addenemy(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let response = petition!(post, "/", enemy_name, ephemeral);
    return match response.status() {
        reqwest::StatusCode::CREATED => Some((
            "Enemy registered on the system correctly. Do not forget to reveal it, if necessary."
                .to_string(),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn enemy(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let response = petition!(get, "/", enemy_name, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "{}/{}",
                env::var("SERVER_EXTERNAL_URL")
                    .expect("SERVER_INTERNAL_URL environmental variable not set."),
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: enemy)."
                )
            ),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn setbasics(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let traits = get_cmd_opt!(options, 5, String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let basics = EnemyBasicsForm {
        enemy_type: get_cmd_opt!(options, 1, String).to_string(),
        hp: get_cmd_opt!(options, 2, Integer) as i16,
        ac: get_cmd_opt!(options, 3, Integer) as u8,
        mov: get_cmd_opt!(options, 4, Integer) as u8,
        traits,
    };

    let response = petition!(post, format!("/{}/basics", enemy_name), basics, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's basic information.".to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's basic information because some data is unknown
                    to the system. Unrecognized item: **{}**.",
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: setbasics)."
                )
            ),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn setattrs(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let attrs = EnemyAttributesForm {
        str: get_cmd_opt!(options, 1, Integer) as u8,
        dex: get_cmd_opt!(options, 2, Integer) as u8,
        con: get_cmd_opt!(options, 3, Integer) as u8,
        int: get_cmd_opt!(options, 4, Integer) as u8,
        wis: get_cmd_opt!(options, 5, Integer) as u8,
        cha: get_cmd_opt!(options, 6, Integer) as u8,
        str_sav: get_cmd_opt!(options, 7, Integer) as u8,
        dex_sav: get_cmd_opt!(options, 8, Integer) as u8,
        con_sav: get_cmd_opt!(options, 9, Integer) as u8,
        int_sav: get_cmd_opt!(options, 10, Integer) as u8,
        wis_sav: get_cmd_opt!(options, 11, Integer) as u8,
        cha_sav: get_cmd_opt!(options, 12, Integer) as u8,
    };

    let response = petition!(post, format!("/{}/attribues", enemy_name), attrs, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's ability modifiers.".to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn setskills(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let skills = get_cmd_opt!(options, 1, String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let response = petition!(post, format!("/{}/skills", enemy_name), skills, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's skills.".to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn setriv(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let resistances = get_cmd_opt!(options, 1, String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let immunities = get_cmd_opt!(options, 2, String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let vulnerabilities = get_cmd_opt!(options, 3, String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let riv = EnemyRIVForm {
        resistances,
        immunities,
        vulnerabilities,
    };

    let response = petition!(post, format!("/{}/riv", enemy_name), riv, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's resistances, immunities, and vulnerabilities."
                .to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's resistances, immunities, and vulnerabilities
                    because some data is unknown to the system. Unrecognized item: **{}**.",
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: setriv)."
                )
            ),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn setabilitytrees(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let tree = get_cmd_opt!(options, 1, String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let response = petition!(
        post,
        format!("/{}/ability_trees", enemy_name),
        tree,
        ephemeral
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's ability trees.".to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn addability(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let ability_name = get_cmd_opt!(options, 1, String).to_string();

    let ability = EnemyAbilityForm {
        name: ability_name.clone(),
        description: get_cmd_opt!(options, 2, String).to_string(),
        tree: get_cmd_opt!(options, 3, String).to_string(),
    };

    let response = petition!(post, format!("/{}/ability", enemy_name), ability, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Correctly updated the enemy's {} ability.", ability_name),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's resistances, immunities, and vulnerabilities
                    because some data is unknown to the system. Unrecognized item: **{}**.",
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: setability)."
                )
            ),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn addnote(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let note = get_cmd_opt!(options, 1, String).to_string();

    let response = petition!(post, format!("/{}/note", enemy_name), note);
    return match response.status() {
        reqwest::StatusCode::OK => {
            Some(("Correctly added the note to the enemy.".to_string(), false))
        }
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response),
    };
}

pub async fn delnote(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let note_idx = get_cmd_opt!(options, 1, Integer);

    let response = petition!(delete, format!("/{}/note", enemy_name), note_idx);
    return match response.status() {
        reqwest::StatusCode::OK => Some((format!("Correctly deleted note number {} from the enemy. The remining notes might have been reordered.", note_idx), false)),
        reqwest::StatusCode::NOT_FOUND => Some((
            NOT_FOUND_MSG.to_string(),
            false,
        )),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealenemy(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = sanitize_name(get_cmd_opt!(options, 0, String));

    let response = petition!(post, format!("/{}/reveal", enemy_name));
    return match response.status() {
        reqwest::StatusCode::CREATED => Some((
            format!(
                "Enemy revealed: {}",
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: revealenemy)."
                )
            ),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealbasics(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(options, 0, String);

    let response = petition!(
        post,
        format!("/{}/reveal/basics", sanitize_name(enemy_name))
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Revealed {}'s basic information on its page.", enemy_name),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealattrs(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(options, 0, String);

    let response = petition!(post, format!("/{}/reveal/attrs", sanitize_name(enemy_name)));
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Revealed {}'s ability modifiers on its page", enemy_name),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealskills(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(options, 0, String);

    let response = petition!(
        post,
        format!("/{}/reveal/skills", sanitize_name(enemy_name))
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Revealed {}'s skills on its page", enemy_name),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealriv(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(options, 0, String);

    let response = petition!(post, format!("/{}/reveal/riv", sanitize_name(enemy_name)));
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Revealed {}'s resistances, immunities, and vulnerabilities on its page",
                enemy_name
            ),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}
pub async fn revealability(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(options, 0, String);

    let ability_name = get_cmd_opt!(options, 2, String).to_string();

    let ability = EnemyAbilityForm {
        tree: get_cmd_opt!(options, 1, String).to_string(),
        name: ability_name.clone(),
        description: "".to_string(),
    };

    let response = petition!(
        post,
        format!("/{}/reveal/ability", sanitize_name(enemy_name)),
        ability
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Revealed {}'s {} ability on its page",
                enemy_name, ability_name
            ),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's abilities because some data is unknown to the system.
                Unrecognized item: **{}**.\n
                (Maybe the ability was not previously added to the system using `/addability`?)",
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: setability)."
                )
            ),
            false,
        )),
        _ => unexpected_response!(response, false),
    };
}

pub async fn addriveffect(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);

    let name = get_cmd_opt!(options, 0, String).to_string();
    let category = get_cmd_opt!(options, 1, String).to_string();

    let riv_effect = RivEffect {
        name: name.clone(),
        category: category.clone(),
    };

    let response = petition!(post, "/riv", riv_effect, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Correctly added RIV effect {} of type {} to the system.",
                name, category
            ),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn addtrait(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, last, Boolean, true);

    let name = get_cmd_opt!(options, 0, String);
    let category = get_cmd_opt!(options, 1, String);
    let subcategory = get_cmd_opt!(options, 2, String);

    let t = Trait {
        name: name.to_string(),
        category: category.to_string(),
        subcategory: subcategory.to_string(),
        description: get_cmd_opt!(options, 3, String).to_string(),
    };

    let response = petition!(post, "/trait", t, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Correctly added trait {} in category {}, subcategory {}, to the system.",
                name, category, subcategory
            ),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub fn register() -> Vec<CreateCommand> {
    let mut commands = Vec::<CreateCommand>::with_capacity(18);
    commands.push(
        CreateCommand::new("addenemy")
            .description("Add an enemy to the bestiary.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy to create.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("enemy")
            .description("Get the URL for an enemy on the bestiary.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setbasics")
            .description("Set basic information (HP, AC, movement and traits) for an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "type",
                "The type, or short description, of this enemy as a living being.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "HP",
                "The HP of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "AC",
                "The AC of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Mov",
                "The movement speed of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "Traits",
                "Comma-separated list of the traits of the enemy.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setattrs")
            .description("Set the ability modifiers of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "STR",
                "Strenth modifier.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "DEX",
                "Dexterity modifier.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "CON",
                "Constitution modifier.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "INT",
                "Intelligence modifier.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "WIS",
                "Wisdom modifier.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "CHA",
                "Charisma modifier.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Saving STR",
                "Strenth modifier Saving fors.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Saving DEX",
                "Dexterity modifier Saving fors.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Saving CON",
                "Constitution modifier Saving fors.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Saving INT",
                "Intelligence modifier Saving fors.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Saving WIS",
                "Wisdom modifier Saving fors.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "Saving CHA",
                "Charisma modifier Saving fors.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setskills")
            .description("Set the skills of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "skills",
                "Comma-separated list of the names of the skills of the enemy.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setriv")
            .description("Set the resistances, immunities and vulnerabilities of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "resistances",
                "Comma-separated list of the resistances of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "immunities",
                "Comma-separated list of the immunities of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "vulnerability",
                "Comma-separated list of the vulnerabilities of the enemy.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setabilitytrees")
            .description("Set the names of the ability trees of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "ability tree names",
                "Comma-separated list of the names of the ability trees of the enemy.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("addability")
            .description("Add an ability to an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "ability name",
                "The name of the ability to add to the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "description",
                "The description of the ability to add.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "ability tree",
                "The name of the ability tree the ability belongs to.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("addnote")
            .description("Add a note to an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "note",
                "The note to add.",
            )),
    );
    commands.push(
        CreateCommand::new("delnote")
            .description("Remove a note from an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::Integer,
                "note number",
                "The number of the note to remove, as specified on the enemy's page.",
            )),
    );
    commands.push(
        CreateCommand::new("revealenemy")
            .description("Reveal an enemy and make it available on the encyclopeida.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy.",
            )),
    );
    commands.push(
        CreateCommand::new("revealbasics")
            .description("Reveal the basic information (HP, AC, Mov) of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy.",
            )),
    );
    commands.push(
        CreateCommand::new("revealattrs")
            .description("Reveal the ability modifiers (attributes/stats) of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy.",
            )),
    );
    commands.push(
        CreateCommand::new("revealskills")
            .description("Reveal the skills of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy.",
            )),
    );
    commands.push(
        CreateCommand::new("revealriv")
            .description("Reveal the resistances, immunities and vulnerabilities of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the enemy.",
            )),
    );
    commands.push(
        CreateCommand::new("revealability")
            .description("Reveal an ability of an enemy.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "enemy name",
                "The name of the enemy.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "tree name",
                "The name of the ability tree to which the ability to reveal belongs.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "ability name",
                "The name of the ability to reveal.",
            )),
    );
    commands.push(
        CreateCommand::new("addriveffect")
            .description(
                "Add a new effect susceptible of resistance, immunity or vulnerability
                 to the system.",
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the effect.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "type",
                    "The type of the effect.",
                )
                .add_string_choice("Damage RIV", "dmg")
                .add_string_choice("Condition RIV", "condition"),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("addtrait")
            .description("Add a new trait for enemies.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "The name of the trait.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "category",
                "The category the trait belongs to.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "category",
                "The subcategory inside the category the trait belongs to.",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "description",
                "The description of the trait.",
            ))
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = true).",
                )
                .required(false),
            ),
    );

    commands
}
