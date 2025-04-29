/*
 *  Denedé: Discord bot for generating D&D dice rolls, written in Rust.
 *  Copyright (C) 2023-2025  Bolu <bolu@tuta.io>
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
use std::fs;
use std::path::Path;

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
    ($options:ident, $name:expr, $type:ident) => {{
        let ResolvedOption {
            value: ResolvedValue::$type(tmp),
            ..
        } = $options
            .iter()
            .find(|ResolvedOption { name: name, .. }| name == &$name)
            .expect(
                format!(
                    "Could not get option in {}.{}.",
                    stringify!($options),
                    $name
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
    /* Get an option that may exist from the list of passed options. */
    ($options:ident, $name:expr, $type:ident, $default:expr) => {{
        if let Some(ResolvedOption {
            value: ResolvedValue::$type(tmp),
            ..
        }) = $options
            .iter()
            .find(|ResolvedOption { name: name, .. }| name == &$name)
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
    // TODO: Check proper slash usage.
    ($end:expr) => {
        format!(
            "{}{}",
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
macro_rules! request {
    ($req_type:ident, $endpoint:expr, $data:expr, $ephemeral:expr) => {{
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
    ($req_type:ident, $endpoint:expr, $data:expr) => {{
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
fn sanitize_name<Stringlike: AsRef<str>>(name: Stringlike) -> String {
    name.as_ref().to_lowercase().replace(" ", "_")
}

/* Command functions: */

const NOT_FOUND_MSG: &str = "Could not find the specified enemy on the system. \
(Or something very wrong happened to the server.)";

pub async fn url(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);

    Some((
        env::var("SERVER_EXTERNAL_URL")
            .expect("SERVER_EXTERNAL_URL environmental variable not set."),
        ephemeral,
    ))
}

pub async fn addenemy(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(options, "name", String);

    let response = request!(post, "/enemy/", enemy_name, ephemeral);
    return match response.status() {
        reqwest::StatusCode::CREATED => Some((
            "Enemy registered on the system correctly. Do not forget to reveal it, if necessary."
                .to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::FORBIDDEN => Some((
            "Seems like an enemy with that name already exists on the encyclopedia.".to_string(),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn enemy(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "name",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let response = request!(get, "/enemy/", enemy_name, ephemeral);
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
        reqwest::StatusCode::FORBIDDEN => Some((
            "The specified enemy is currently unavailable.".to_string(),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn target(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, false);
    let enemy_name = get_cmd_opt!(options, "name", String);

    // Check that the enemy exists:
    let response = request!(get, "/enemy/", enemy_name, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK | reqwest::StatusCode::FORBIDDEN => {
            fs::write(".target_name", enemy_name).expect("Could not write into .target_name.");
            Some((
                format!(
                    "Now all bot commands will target {} by default. \
                     You can still explicitly specify another creature to target on \
                     individual commands.",
                    enemy_name
                ),
                ephemeral,
            ))
        }
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub fn gettarget(options: &[ResolvedOption]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, false);

    if !Path::new(".target_name").exists() {
        return Some((
            "There is no active target on the system.".to_string(),
            ephemeral,
        ));
    }

    let target = fs::read_to_string(".target_name").expect("Could not read .target_name.");

    Some((format!("The current target is **{}**.", target), ephemeral))
}

pub async fn setbasics(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let traits = get_cmd_opt!(options, "traits", String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let basics = EnemyBasicsForm {
        enemy_type: get_cmd_opt!(options, "type", String).to_string(),
        hp: get_cmd_opt!(options, "hp", Integer) as i16,
        ac: get_cmd_opt!(options, "ac", Integer) as u8,
        mov: get_cmd_opt!(options, "mov", Integer) as u8,
        traits,
    };

    let response = request!(
        post,
        format!("/enemy/{}/basics", enemy_name),
        basics,
        ephemeral
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's basic information.".to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's basic information because some data is unknown \
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
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let attrs = EnemyAttributesForm {
        str: get_cmd_opt!(options, "str", Integer) as u8,
        dex: get_cmd_opt!(options, "dex", Integer) as u8,
        con: get_cmd_opt!(options, "con", Integer) as u8,
        int: get_cmd_opt!(options, "int", Integer) as u8,
        wis: get_cmd_opt!(options, "wis", Integer) as u8,
        cha: get_cmd_opt!(options, "cha", Integer) as u8,
        str_sav: get_cmd_opt!(options, "saving_str", Integer) as u8,
        dex_sav: get_cmd_opt!(options, "saving_dex", Integer) as u8,
        con_sav: get_cmd_opt!(options, "saving_con", Integer) as u8,
        int_sav: get_cmd_opt!(options, "saving_int", Integer) as u8,
        wis_sav: get_cmd_opt!(options, "saving_wis", Integer) as u8,
        cha_sav: get_cmd_opt!(options, "saving_cha", Integer) as u8,
    };

    let response = request!(
        post,
        format!("/enemy/{}/attributes", enemy_name),
        attrs,
        ephemeral
    );
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
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let skills = get_cmd_opt!(options, "skills", String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let response = request!(
        post,
        format!("/enemy/{}/skills", enemy_name),
        skills,
        ephemeral
    );
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
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let resistances = get_cmd_opt!(options, "resistances", String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let immunities = get_cmd_opt!(options, "immunities", String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let vulnerabilities = get_cmd_opt!(options, "vulnerabilities", String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();
    let riv = EnemyRIVForm {
        resistances,
        immunities,
        vulnerabilities,
    };

    let response = request!(post, format!("/enemy/{}/riv", enemy_name), riv, ephemeral);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly updated the enemy's resistances, immunities, and vulnerabilities."
                .to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's resistances, immunities, and vulnerabilities \
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
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let tree = get_cmd_opt!(options, "ability_tree_names", String)
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let response = request!(
        post,
        format!("/enemy/{}/ability_trees", enemy_name),
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
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let ability_name = get_cmd_opt!(options, "ability_name", String).to_string();

    let ability = EnemyAbilityForm {
        name: ability_name.clone(),
        tree: get_cmd_opt!(options, "ability_tree", String).to_string(),
        description: get_cmd_opt!(options, "description", String, "").to_string(),
    };

    let response = request!(
        post,
        format!("/enemy/{}/ability", enemy_name),
        ability,
        ephemeral
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Correctly updated the enemy's {} ability.", ability_name),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's resistances, immunities, and vulnerabilities \
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
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let note = get_cmd_opt!(options, "note", String).to_string();

    let response = request!(post, format!("/enemy/{}/note", enemy_name), note);
    return match response.status() {
        reqwest::StatusCode::OK => {
            Some(("Correctly added the note to the enemy.".to_string(), false))
        }
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response),
    };
}

pub async fn delnote(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let note_idx = get_cmd_opt!(options, "note_number", Integer);

    let response = request!(delete, format!("/enemy/{}/note", enemy_name), note_idx);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Correctly deleted note number {} from the enemy. \
                    The remining notes might have been reordered.",
                note_idx
            ),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn setimage(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let file = get_cmd_opt!(options, "image", Attachment);

    if file.content_type == Some("image".to_string()) {
        return Some((
            "The attachment sent is not recognized as an image.".to_string(),
            ephemeral,
        ));
    }

    let response = request!(post, format!("/enemy/{}/image", enemy_name), file.url);
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            "Correctly uploaded the enemy's image.".to_string(),
            ephemeral,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), ephemeral)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not set the enemy's image because some data is erroneous. Error: **{}**.",
                response.text().await.expect(
                    "Could not decode a server's response's body as text (command: setimage)."
                )
            ),
            ephemeral,
        )),
        _ => unexpected_response!(response, ephemeral),
    };
}

pub async fn revealenemy(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );
    let enemy_name = sanitize_name(enemy_name);

    let response = request!(post, format!("/enemy/{}/reveal", enemy_name));
    return match response.status() {
        reqwest::StatusCode::CREATED => Some((
            format!(
                "Enemy revealed: {}/{}",
                env::var("SERVER_EXTERNAL_URL")
                    .expect("SERVER_INTERNAL_URL environmental variable not set."),
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
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let response = request!(
        post,
        format!("/enemy/{}/reveal/basics", sanitize_name(enemy_name))
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
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let response = request!(
        post,
        format!("/enemy/{}/reveal/attrs", sanitize_name(enemy_name))
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Revealed {}'s ability modifiers on its page.", enemy_name),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealskills(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let response = request!(
        post,
        format!("/enemy/{}/reveal/skills", sanitize_name(enemy_name))
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!("Revealed {}'s skills on its page.", enemy_name),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn revealriv(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let response = request!(
        post,
        format!("/enemy/{}/reveal/riv", sanitize_name(enemy_name))
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Revealed {}'s resistances, immunities, and vulnerabilities on its page.",
                enemy_name
            ),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}
pub async fn revealability(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let ability_name = get_cmd_opt!(options, "ability_name", String).to_string();

    let ability = EnemyAbilityForm {
        tree: get_cmd_opt!(options, "tree_name", String).to_string(),
        name: ability_name.clone(),
        description: "".to_string(),
    };

    let response = request!(
        post,
        format!("/enemy/{}/reveal/ability", sanitize_name(enemy_name)),
        ability
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((
            format!(
                "Revealed {}'s {} ability on its page.",
                enemy_name, ability_name
            ),
            false,
        )),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        reqwest::StatusCode::BAD_REQUEST => Some((
            format!(
                "Could not update the enemy's abilities because some data is unknown to the system. \
                Unrecognized item: **{}**.\n\
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

pub async fn refresh(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);

    let enemy_name = get_cmd_opt!(
        options,
        "enemy",
        String,
        &fs::read_to_string(".target_name").expect("Could not read .target_name.")
    );

    let response = request!(
        post,
        format!("/enemy/{}/refresh", sanitize_name(enemy_name))
    );
    return match response.status() {
        reqwest::StatusCode::OK => Some((format!("Refreshed {}'s page.", enemy_name,), ephemeral)),
        reqwest::StatusCode::NOT_FOUND => Some((NOT_FOUND_MSG.to_string(), false)),
        _ => unexpected_response!(response, false),
    };
}

pub async fn addriveffect(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);

    let name = get_cmd_opt!(options, "name", String).to_string();
    let category = get_cmd_opt!(options, "type", String).to_string();

    let riv_effect = RivEffect {
        name: name.clone(),
        category: category.clone(),
    };

    let response = request!(post, "/riv", riv_effect, ephemeral);
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
    let ephemeral = get_cmd_opt!(options, "hidden", Boolean, true);

    let name = get_cmd_opt!(options, "name", String);
    let category = get_cmd_opt!(options, "category", String);
    let subcategory = get_cmd_opt!(options, "subcategory", String);

    let t = Trait {
        name: name.to_string(),
        category: category.to_string(),
        subcategory: subcategory.to_string(),
        description: get_cmd_opt!(options, "description", String).to_string(),
    };

    let response = request!(post, "/trait", t, ephemeral);
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
    let mut commands = Vec::<CreateCommand>::with_capacity(23);
    commands.push(
        CreateCommand::new("url")
            .description("[+N] Get the URL for the encyclopedia.")
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
        CreateCommand::new("addenemy")
            .description("[+N] Add an enemy to the bestiary.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The name of the enemy to create.",
                )
                .required(true),
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
        CreateCommand::new("enemy")
            .description("[+N] Get the URL for an enemy on the bestiary.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("target")
            .description(
                "[+N] Make a creature the default target of all subsequent commands. \
                (I.e., cache the name.)",
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The name of the creature to target automatically.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = false).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("gettarget")
            .description("[+N] Get the name of the current default target creature on the system.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "hidden",
                    "Hide the command's response to other users (default = false).",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setbasics")
            .description("[+N] Set basic information (HP, AC, movement and traits) for an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "type",
                    "The type, or short description, of this enemy as a living being.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "hp", "The HP of the enemy.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "ac", "The AC of the enemy.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "mov",
                    "The movement speed of the enemy.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "traits",
                    "Comma-separated list of the traits of the enemy.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("setattrs")
            .description("[+N] Set the ability modifiers of an enemy.")
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "str", "Strenth modifier.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "dex", "Dexterity modifier.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "con",
                    "Constitution modifier.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "int",
                    "Intelligence modifier.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "wis", "Wisdom modifier.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "cha", "Charisma modifier.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "saving_str",
                    "Strenth modifier for saving throws.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "saving_dex",
                    "Dexterity modifier for saving throws.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "saving_con",
                    "Constitution modifier for saving throws.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "saving_int",
                    "Intelligence modifier for saving throws.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "saving_wis",
                    "Wisdom modifier for saving throws.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "saving_cha",
                    "Charisma modifier for saving throws.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("setskills")
            .description("[+N] Set the skills of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "skills",
                    "Comma-separated list of the names of the skills of the enemy.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("setriv")
            .description("[+N] Set the resistances, immunities and vulnerabilities of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "resistances",
                    "Comma-separated list of the resistances of the enemy.\nIf the enemy does not have resistances, specify only \"none\".",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "immunities",
                    "Comma-separated list of the immunities of the enemy.\nIf the enemy does not have immunities, specify only \"none\".",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "vulnerabilities",
                    "Comma-separated list of the vulnerabilities of the enemy.\nIf the enemy does not have vulnerabilities, specify only \"none\".",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("setabilitytrees")
            .description("[+N] Set the names of the ability trees of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "ability_tree_names",
                    "Comma-separated list of the names of the ability trees of the enemy.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("addability")
            .description("[+N] Add an ability to an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "ability_tree",
                    "The name of the ability tree the ability belongs to.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "ability_name",
                    "The name of the ability to add to the enemy.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "description",
                    "The description of the ability to add.",
                )
                .required(false),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("addnote")
            .description("[+N] Add a note to an enemy.")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "note", "The note to add.")
                    .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("delnote")
            .description("[+N] Remove a note from an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "note_number",
                    "The number of the note to remove, as specified on the enemy's page.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("setimage")
            .description("[+N] Set an image for an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Attachment,
                    "image",
                    "The image to set.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
    // TODO: use subcommands for reveals (serenity.rs documentation for that API part is somewhat
    // vague, and I found no examples).
    commands.push(
        CreateCommand::new("revealenemy")
            .description("[+N] Reveal an enemy and make it available on the encyclopeida.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("revealbasics")
            .description("[+N] Reveal the basic information (HP, AC, Mov) of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("revealattrs")
            .description("[+N] Reveal the ability modifiers (attributes/stats) of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("revealskills")
            .description("[+N] Reveal the skills of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("revealriv")
            .description("[+N] Reveal the resistances, immunities and vulnerabilities of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("revealability")
            .description("[+N] Reveal an ability of an enemy.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "tree_name",
                    "The name of the ability tree to which the ability to reveal belongs.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "ability_name",
                    "The name of the ability to reveal.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
            ),
    );
    commands.push(
        CreateCommand::new("refresh")
            .description("[+N] Refresh an enemy's page.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "enemy",
                    "The name of the enemy.",
                )
                .required(false),
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
        CreateCommand::new("addriveffect")
            .description(
                "[+N] Add a new effect susceptible of resistance, immunity or \
                vulnerability to the system.",
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The name of the effect.",
                )
                .required(true),
            )
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
            .description("[+N] Add a new trait for enemies.")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "The name of the trait.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "category",
                    "The category the trait belongs to.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "subcategory",
                    "The subcategory inside the category the trait belongs to.",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "description",
                    "The description of the trait.",
                )
                .required(true),
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

    commands
}
