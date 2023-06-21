use rand::Rng;
use super::player::{Player, PlayerAction, PlayerConfig};

pub fn get_action(player: &Player) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    match num {
        0..=20 => PlayerAction::Fold,
        21..=55 => PlayerAction::Check,
        56..=70 => {
            let amount: u32 = if player.money <= 100 {
                // just go all in if we are at 10% starting

                player.money
            } else {
                rand::thread_rng().gen_range(1..player.money / 2_u32)
            };
            PlayerAction::Bet(amount)
        }
        _ => PlayerAction::Call
    }
}
