mod logic;

fn main() {
    println!("Hello, world!");    
    let mut game = logic::Game::new();
    let num_bots = 5;
    for i in 0..num_bots {
	let name = format!("Mr {}", i);
	game.add_bot(name);
    }
    let user_name = "Adam".to_string();
    game.add_user(user_name);
    game.play();
}

