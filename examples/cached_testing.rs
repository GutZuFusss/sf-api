use std::time::Instant;

use enum_map::EnumMap;
use sf_api::{
    gamestate::{dungeons::LightDungeon, GameState},
    misc::EnumMapGet,
    session::*,
    simulate::{Battle, BattleEvent, BattleFighter, BattleLogger, BattleSide, BattleTeam, PlayerFighterSquad, UpgradeableFighter},
    sso::SFAccount,
};
use strum::IntoEnumIterator;

#[tokio::main]
pub async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    const SSO: bool = true;
    const USE_CACHE: bool = true;

    let custom_resp: Option<&str> = None;
    let command = None;

    let username = std::env::var("USERNAME").unwrap();

    let mut session = match SSO {
        true => SFAccount::login(
            std::env::var("SSO_USERNAME").unwrap(),
            std::env::var("PASSWORD").unwrap(),
        )
        .await
        .unwrap()
        .characters()
        .await
        .unwrap()
        .into_iter()
        .flatten()
        .find(|a| a.username() == username)
        .unwrap(),
        false => Session::new(
            &username,
            &std::env::var("PASSWORD").unwrap(),
            ServerConnection::new(&std::env::var("SERVER").unwrap()).unwrap(),
        ),
    };

    _ = std::fs::create_dir("cache");
    let cache_name = format!("cache/{username}.login");

    let login_data = match (USE_CACHE, std::fs::read_to_string(&cache_name)) {
        (true, Ok(s)) => serde_json::from_str(&s).unwrap(),
        _ => {
            let login_data = session.login().await.unwrap();
            let ld = serde_json::to_string_pretty(&login_data).unwrap();
            std::fs::write(&cache_name, ld).unwrap();
            login_data
        }
    };

    let mut gd = GameState::new(login_data).unwrap();

    if let Some(resp) = custom_resp {
        let bruh = Response::parse(
            resp.to_string(),
            chrono::Local::now().naive_local(),
        )
        .unwrap();
        gd.update(bruh).unwrap();
    }

    let Some(command) = command else {
        let js = serde_json::to_string_pretty(&gd).unwrap();
        std::fs::write("character.json", js).unwrap();

        let uf = UpgradeableFighter {
            is_companion: false,
            level: 1,
            class: Default::default(),
            attribute_basis: Default::default(),
            _attributes_bought: Default::default(),
            pet_attribute_bonus_perc: Default::default(),
            equipment: Default::default(),
            active_potions: Default::default(),
            portal_hp_bonus: 0,
            portal_dmg_bonus: 0,
        };

        let squad = PlayerFighterSquad::new(&gd);
        let player = BattleFighter::from_upgradeable(&squad.character);
        let mut player_squad = [player];
        for dungeon in LightDungeon::iter() {
            let Some(monster) = gd.dungeons.current_enemy(dungeon) else {
                continue;
            };
            let monster = BattleFighter::from_monster(monster);
            let mut monster = [monster];
            let mut battle = Battle::new(&mut player_squad, &mut monster);

            for a in 0..3 {
                battle.simulate_turn(&mut ());
                let right_hp = battle.right.current().unwrap().current_hp;
                let left_hp = battle.left.current().unwrap().current_hp;
                println!("{} Right HP: {}, Left HP: {}", a, right_hp, left_hp);
                println!("{:?}", battle.left.current().unwrap())
            }
        }

        return;
    };
    let cache_name = format!(
        "cache/{username}-{}.response",
        serde_json::to_string(&command).unwrap()
    );

    let resp = match (USE_CACHE, std::fs::read_to_string(&cache_name)) {
        (true, Ok(s)) => serde_json::from_str(&s).unwrap(),
        _ => {
            let resp = session.send_command_raw(&command).await.unwrap();
            let ld = serde_json::to_string_pretty(&resp).unwrap();
            std::fs::write(cache_name, ld).unwrap();
            resp
        }
    };

    gd.update(&resp).unwrap();
    let js = serde_json::to_string_pretty(&gd).unwrap();
    std::fs::write("character.json", js).unwrap();
}
