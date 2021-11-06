use std::iter;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::{HashSet, HashMap};
use std::cmp::Ordering;

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, EnumIter, Hash)]
enum Rank {
    TWO = 2,
    THREE = 3,
    FOUR = 4,
    FIVE = 5,
    SIX = 6,
    SEVEN = 7,
    EIGHT = 8,
    NINE = 9,
    TEN = 10,
    JACK = 11,
    QUEEN = 12,
    KING = 13,
    ACE = 14,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, EnumIter)]
enum Suit {
    CLUB,
    DIAMOND,
    HEART,
    SPADE
}

#[derive(Eq, Debug, Copy, Clone)]
struct Card {
    rank: Rank,
    suit: Suit,
}

/// We simply compare Cards based on their rank field.
impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank.cmp(&other.rank)
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank
    }
}


#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, EnumIter)]
enum HandRanking {
    HIGHCARD = 1,
    PAIR = 2,    
    TWOPAIR = 3,
    THREEOFAKIND = 4,
    STRAIGHT = 5,
    FLUSH = 6,
    FULLHOUSE = 7,
    FOUROFAKIND = 8,
    STRAIGHTFLUSH = 9,
    ROYALFLUSH = 10,
}

/// The hand result has the HandRanking, for quick comparisons, then the cads that make
/// up that HandRanking, along with the remaining kicker cards for tie breaking (sorted)
/// There is also a field "value", which gives a value of the hand that can be used to quickly
/// compare it against other hands. The HandRanking, then each of the constituent cards, then the kickers,
/// are each represented by 4 bits, so a better hand will have a higher value.
/// e.g. a hand of Q, Q, Q, 9, 4 would look like
/// {
/// hand_ranking: HandRanking::THREEOFAKIND,
/// contsituent_cards: [Q, Q, Q],
/// kickers: [9, 4]
/// value = [3] | [12] | [12] | [12] | [9] | [4] == [0000] | [0000] | [0011] | [1100] | [1100] | [1100] | [1001] | [0100]
/// Note: value has eight leading 0s since we only need 24 bits to represent it.
/// }
#[derive(Debug, Eq)]
struct HandResult {
    hand_ranking: HandRanking,
    constituent_cards: Vec<Card>,
    kickers: Vec<Card>,
    value: u32, // the absolute value of this hand, which can be used to compare against another hand
}

/// We simply compare HandResults based on their value field.
impl Ord for HandResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for HandResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HandResult {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl HandResult {

    /// Given a hand ranking, along with the constituent cards and the kickers, this function returns a numerical score
    /// that determines the hand's value compared to any other hand
    fn score_hand (hand_ranking: HandRanking, constituent_cards: & Vec<Card>, kickers: & Vec<Card>) -> u32 {
	let mut value = hand_ranking as u32;
	value = value << 20; // shift it into the most sifnificant area we need
	match hand_ranking {
	    HandRanking::HIGHCARD
		| HandRanking::PAIR
		| HandRanking::THREEOFAKIND
		| HandRanking::STRAIGHT
		| HandRanking::FLUSH
		| HandRanking::FOUROFAKIND
		| HandRanking::STRAIGHT
		| HandRanking::STRAIGHTFLUSH
		| HandRanking::ROYALFLUSH => {
		    // These handrankings are all uniquely identified by a single constituent card
		    // first add the rank of the constituent
		    let mut extra = constituent_cards.last().unwrap().rank as u32;
		    extra = extra << 16;
		    value += extra;
		},
	    HandRanking::TWOPAIR  => {
		// a two pair is valued by its higher pair, then lower pair
		let mut extra = constituent_cards.last().unwrap().rank as u32;
		extra = extra << 16;
		value += extra;

		// the lower pair is sorted to the front
		extra = constituent_cards[0].rank as u32;
		extra = extra << 12;
		value += extra;		    
	    },
	    HandRanking::FULLHOUSE => {
		// a full house is valued first by the three of a kind, then the pair
		// the three of a kind will always exist as the middle element, regardless of the sort order
		let mut extra = constituent_cards[2].rank as u32;
		extra = extra << 16;
		value += extra;

		// the pair will be either at the beginning or the end of the constituent_cards, we need to check.
		// this depends on the sort.
		// e.g. could be [ 2, 2, 2, 6, 6 ], OR [ 2, 2, 6, 6, 6 ]
		let mut second_extra = constituent_cards[0].rank as u32;
		if second_extra == extra {
		    // the first card was the same as the middle card, i.e. we grabbed another card in the three of a kind.
		    // So grab the last card in the list, which will necessarily be part of the pair
		    second_extra = constituent_cards.last().unwrap().rank as u32;
		}
		second_extra = second_extra << 12;
		value += second_extra;
	    }
	}
	
	// next add the value of the kicker(s), in order
	// Note: for rankings without kickers, this loop simply won't happen
	let mut shift_amount = 0;
	// TODO: double check this logic. originally shift_amount started at 12. but that wont work for 2 pair in particular, since
	// the second pair is being shifted 12. so if we start at 0 and go UP, i think we are good right?
	for i in (0..kickers.len()) {
	    let mut extra = kickers[i].rank as u32;
	    extra << shift_amount;
	    value += extra;
	    shift_amount += 4;
	}
	
	value
    }

    /// Given a hand of 5 cards, we return a HandResult, which tells
    /// us the hand ranking, the constituent cards, kickers, and hand score    
    fn analyze_hand(mut five_cards: Vec<Card>) -> Self {
	assert!(five_cards.len() == 5);
	five_cards.sort(); // first sort by Rank
	//println!("five cards = {:?}", five_cards);
	
	let hand_ranking: HandRanking;

	let mut rank_counts: HashMap<Rank, u8> = HashMap::new();	
	let mut is_flush = true;
	let first_suit = five_cards[0].suit;
	let mut is_straight = true;
	let first_rank = five_cards[0].rank as usize;
	for (i, card) in five_cards.iter().enumerate() {
	    let count = rank_counts.entry(card.rank).or_insert(0);
	    *count += 1;
	    if card.suit != first_suit {
		is_flush = false;
	    }
	    if card.rank as usize != first_rank + i {
		// TODO: we need to handle ACE being high or low
		is_straight = false;
	    }
	}

	if is_flush {
	    println!("is_flush = {}", is_flush);
	}
	if is_straight {
	    println!("is_straight = {}", is_straight);
	}
	//println!("rank counts = {:?}", rank_counts);
	let mut constituent_cards = Vec::new();

	let mut kickers = Vec::new();
	
	if is_flush && is_straight {
	    if let Rank::ACE = five_cards[4].rank {
		hand_ranking = HandRanking::ROYALFLUSH;
	    }
	    else {
		hand_ranking = HandRanking::STRAIGHTFLUSH;		    
	    }
	    constituent_cards.extend(five_cards);	    
	} else {
	    let mut num_fours = 0;
	    let mut num_threes = 0;
	    let mut num_twos = 0;	    
	    for (rank, count) in &rank_counts {
		//println!("rank = {:?}, count = {}", rank, count);
		match count {
		    4 => num_fours += 1,
		    3 => num_threes += 1,
		    2 => num_twos += 1,
		    _ => ()
		}
	    }

	    if num_fours == 1 {
		hand_ranking = HandRanking::FOUROFAKIND;
		for card in five_cards {
		    match *rank_counts.get(&card.rank).unwrap()  {
			4 => constituent_cards.push(card),
			_ => kickers.push(card),
		    }
		}				
	    }
	    else if num_threes == 1 && num_twos == 1 {
		hand_ranking = HandRanking::FULLHOUSE;
		for card in five_cards {
		    match *rank_counts.get(&card.rank).unwrap()  {
			2 | 3 => constituent_cards.push(card),
			_ => kickers.push(card),
		    }
		}		
	    }
	    else if is_flush {
		hand_ranking = HandRanking::FLUSH;
		constituent_cards.extend(five_cards);	    		
	    } else if is_straight {
		hand_ranking = HandRanking::STRAIGHT;
		constituent_cards.extend(five_cards);	    		
	    } else if num_threes == 1 {
		hand_ranking = HandRanking::THREEOFAKIND;
		for card in five_cards {
		    match *rank_counts.get(&card.rank).unwrap()  {
			3 => constituent_cards.push(card),
			_ => kickers.push(card),
		    }
		}		
	    } else if num_twos == 2 {
		hand_ranking = HandRanking::TWOPAIR;
		for card in five_cards {
		    match *rank_counts.get(&card.rank).unwrap()  {
			2 => constituent_cards.push(card),
			_ => kickers.push(card),
		    }
		}		
	    } else if num_twos == 1 {
		hand_ranking = HandRanking::PAIR;
		for card in five_cards {
		    match *rank_counts.get(&card.rank).unwrap()  {
			2 => constituent_cards.push(card),
			_ => kickers.push(card),
		    }
		}				
	    } else {
		hand_ranking = HandRanking::HIGHCARD;
		constituent_cards.push(five_cards[4]);
		for &card in five_cards.iter().take(4) {
		    kickers.push(card);
		}
	    }
	}
	constituent_cards.sort();
	kickers.sort();
	let value = HandResult::score_hand(hand_ranking, &constituent_cards, &kickers);
	Self {
	    hand_ranking,
	    constituent_cards,
	    kickers,
	    value
	}
    }
}

struct Deck {
    cards: Vec<Card>,
    top: usize, // index that we deal the next card from
}

impl Deck {
    fn new () -> Self {
	// returns a new unshuffled deck of 52 cards 
	let mut cards = Vec::<Card>::with_capacity(52);
	for rank in Rank::iter() {
	    for suit in Suit::iter() {
		cards.push(Card{rank, suit});
	    }
	}
	Deck {cards, top: 0}
    }

    fn shuffle(&mut self) {
	// shuffle the deck of cards
	self.cards.shuffle(&mut rand::thread_rng());
	self.top = 0;
    }

    fn draw_card(&mut self) -> Option<Card> {
	// take the top card from the deck and move the index of the top of the deck
	if self.top == self.cards.len() {
	    // the deck is exhausted, no card to give
	    None
	}
	else {
	    let card = self.cards[self.top];	    
	    self.top += 1;
	    Some(card)
	}
    }
}



enum PlayerAction {
    Fold,
    Check,
    Bet(f64),
    Call,
    //Raise(u32), // i guess a raise is just a bet really?
}

#[derive(Debug)]
struct Player {
    name: String,
    hole_cards: Vec<Card>,    
    is_active: bool,
    money: f64,
}

impl Player {
    fn new(name: String) -> Self {
	Player {
	    name: name,
	    hole_cards: Vec::<Card>::with_capacity(2),
	    is_active: true,
	    money: 1000.0, // let them start with 1000 for now
	}
    }
    
    fn pay(&mut self, payment: f64) {
	println!("getting paid inside {:?}", self);
	self.money += payment
    }

    fn deactivate(&mut self) {
	self.is_active = false;
    }
}

#[derive(Debug, PartialEq)]
enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    ShowDown
}

struct GameHand<'a> {
    deck: &'a mut Deck,
    players: &'a mut Vec<Player>,
    num_active: usize,
    button_idx: usize, // the button index dictates where the action starts
    street: Street,
    pot: f64, // current size of the pot
    flop: Option<Vec<Card>>,
    turn: Option<Card>,
    river: Option<Card>,
}

impl <'a> GameHand<'a> {
    fn new (deck: &'a mut Deck, players: &'a mut Vec<Player>, button_idx: usize) -> Self {
	let num_active = players.iter().filter(|player| player.is_active).count(); // active to start the hand	    	
	GameHand {
	    deck: deck,
	    players: players,
	    num_active: num_active,	    
	    button_idx: button_idx,
	    street: Street::Preflop,
	    pot: 0.0,
	    flop: None,
	    turn: None,
	    river: None
	}

    }
    
    fn transition(&mut self) {
	match self.street {
	    Street::Preflop => {
	    	self.street = Street::Flop;
		self.deal_flop();
		println!("\nFlop = {:?}", self.flop);
	    },
	    Street::Flop => {
	    	self.street = Street::Turn;
		self.deal_turn();
		println!("\nTurn = {:?}", self.turn);		
	    }
	    Street::Turn => {
	    	self.street = Street::River;
		self.deal_river();
		println!("\nRiver = {:?}", self.river);				
	    }
	    Street::River => {
	    	self.street = Street::ShowDown;
	    }
	    Street::ShowDown => () // we are already in the end street (from players folding during the street)
	}
    }

    
    fn deal_hands(&mut self) {
	for player in self.players.iter_mut() {
	    if player.is_active {
		for _ in 0..2 {
		    if let Some(card) = self.deck.draw_card() {
			player.hole_cards.push(card)
		    } else {
			panic!();
		    }
		}
	    }
	}
    }
    
    fn deal_flop(&mut self) {
	let mut flop = Vec::<Card>::with_capacity(3);
	for _ in 0..3{
	    if let Some(card) = self.deck.draw_card() {
		flop.push(card)
	    } else {
		panic!();
	    }
	}
	self.flop = Some(flop);
    }
    
    fn deal_turn(&mut self) {
	self.turn = self.deck.draw_card();	    
    }
    
    fn deal_river(&mut self) {
	self.river = self.deck.draw_card();	    	    
    }

    fn finish(&mut self) {

	let mut best_indices = HashSet::<usize>::new();	
	if let Street::ShowDown = self.street {
	    // if we made it to show down, there are multiple plauers left, so we need to see who
	    // has the best hand.
	    println!("Multiple active players made it to showdown!");	    
	    let hand_results =  self.players.iter()
		.map(|player| self.determine_best_hand(player)).collect::<Vec<Option<HandResult>>>();

	    let mut best_idx = 0;
	    best_indices.insert(best_idx);	
	    for (mut i, current_result) in hand_results.iter().skip(1).enumerate() {
		i += 1; // increment i to get the actual index, since we are skipping the first element at idx 0
		println!("Index = {}, Current result = {:?}", i, current_result);
		
		if let None = current_result {
		    println!("no hand result at index {:?}", i);
		    continue;
		}
		if *current_result > hand_results[best_idx] {
		    println!("new best hand at index {:?}", i);
		    best_indices.clear();
		    best_indices.insert(i); // only one best hand now
		    best_idx = i;
		}
		else if *current_result == hand_results[best_idx] {
		    println!("equally good hand at index {:?}", i);		    
		    best_indices.insert(i); // another index that also has the best hand
		}
		else {
		    println!("hand worse at index {:?}", i);		    		    
		    continue;
		}
	    }
	} else {
	    for(i, player) in self.players.iter().enumerate() {
		// TODO: make this more functional/rusty
		if player.is_active {
		    println!("found an active player remaining");
		    best_indices.insert(i);
		} else {
		    println!("found an NON active player remaining");
		}
	    }
	    assert!(best_indices.len() == 1); // if we didn't make it to show down, there better be only one player left			    
	    
	}

	// divy the pot to all the winners	
	let num_winners = best_indices.len();
	let payout = self.pot as f64 / num_winners as f64;
	
	for idx in best_indices.iter() {
	    let winning_player = & mut
		self.players[*idx];
	    winning_player.pay(payout);
	}

	// take the players' cards
	for player in self.players.iter_mut() {
	    // todo: is there any issue with calling drain if they dont have any cards?
	    player.hole_cards.drain(..);		
	}
    }


    /// Given a player, we need to determine which 5 cards make the best hand for this player
    fn determine_best_hand(&self, player: &Player) -> Option<HandResult> {
	if !player.is_active {
	    // if the player isn't active, then can't have a best hand
	    return None;
	}
	

	if let Street::ShowDown = self.street {
	    // we look at all possible 7 choose 5 (21) hands from the hole cards, flop, turn, river
	    let mut best_result: Option<HandResult> = None;
	    let mut hand_count = 0;
	    for exclude_idx1 in 0..7 {
		//println!("exclude 1 = {}", exclude_idx1);
		for exclude_idx2 in exclude_idx1+1..7 {
		    //println!("exclude 2 = {}", exclude_idx2);		    
		    let mut possible_hand = Vec::with_capacity(5);
		    hand_count += 1;
		    for (idx, card) in player.hole_cards.iter()
			.chain(self.flop.as_ref().unwrap().iter())
			.chain(iter::once(&self.turn.unwrap()))
			.chain(iter::once(&self.river.unwrap())).enumerate() {
			    if idx != exclude_idx1 && idx != exclude_idx2  {
				//println!("pushing!");
				possible_hand.push(*card);
			    }
			}
		    // we have built a hand of five cards, now evaluate it
		    let current_result = HandResult::analyze_hand(possible_hand);
		    match best_result {
			None => best_result = Some(current_result),			
			Some(result) if current_result > result  => best_result = Some(current_result),
			_ => ()
		    }
		}
	    }
	    assert!(hand_count == 21); // 7 choose 5
	    println!("Looked at {} possible hands", hand_count);
	    println!("player = {}", player.name);
	    println!("best result = {:?}", best_result);
	    best_result
	} else {
	    None
	}
	    
    }
        
    fn play(&mut self) {
	println!("inside of play()");
	self.deck.shuffle();
	//for card in self.deck.cards.iter() {
	//   println!("{:?}", card);
	//}
	self.deal_hands();
	
	println!("self.players = {:?}", self.players);	
	while self.street != Street::ShowDown {
	    //println!("\nStreet is {:?}", self.street);
	    self.play_street();
	    if self.num_active == 1 {
		// if the game is over from players folding
		break;
	    } else {
		// otherwise we move to the next street
		self.transition();
	    }
 	}
	// now we finish up and pay the pot to the winner
	self.finish();	
    }

    fn get_starting_idx(&self) -> usize {
	// the starting index is either the person one more from the button on most streets,
	// or 3 down on the preflop (since the blinds already had to buy in)
	// TODO: this needs to be smarter in small games
	let mut starting_idx = self.button_idx + 1;
	if starting_idx as usize >= self.players.len() {
	    starting_idx += 1;
	}
	starting_idx
    }
    
    fn play_street(&mut self) {
	let mut street_bet: f64 = 0.0;
	let mut cumulative_bets = vec![0.0; self.players.len()]; // each index keeps track of that players' contribution this street

	// TODO: if preflop then collect blinds
	/*
	if self.street == Street::Preflop {
	    let (left, right) = self.players.split_at_mut(starting_idx);
	    for (i, mut player) in right.iter_mut().chain(left.iter_mut()).enumerate() {
	    }
	}
	 */
	
	let starting_idx = self.get_starting_idx(); // which player starts the betting
	let mut num_settled = 0; // keeps track of how many players have either checked through or called the last bet (or made the last bet)
	// if num_settled == self.active, then we are good to go to the next street 
	
	let mut _loop_count = 0;
	'street: loop {
	    /*
	    if loop_count > 2 {
		break;
	    }
	     */
	    // loop_count += 1;
	    //println!("loop count = {}", loop_count);
	    
	    // iterate over the players from the starting index to the end of the vec, and then from the beginning back to the starting index
	    let (left, right) = self.players.split_at_mut(starting_idx);
	    for (i, mut player) in right.iter_mut().chain(left.iter_mut()).enumerate() {
		let player_cumulative = cumulative_bets[i];
		println!("Player = {:?}, i = {}", player, i);
		println!("Current pot = {:?}, Current size of the bet = {:?}, and this player has put in {:?} so far",
			 self.pot,
			 street_bet,
			 player_cumulative);
		if player.is_active {
		    //println!("Player is active");		    
		    // this loop can keep going while it waits for a proper action
		    // get an validate an action from the player
		    match GameHand::get_and_validate_action(&player, street_bet, player_cumulative) {
			PlayerAction::Fold => {
			    println!("Player {:?} folds!", player.name);
			    player.deactivate();	    
			    self.num_active -= 1;
			}
			PlayerAction::Check => {
			    println!("Player checks!");			    			    
			    num_settled += 1;
			},
			PlayerAction::Call => {
			    println!("Player calls!");			    			    			    
			    let difference = street_bet - player_cumulative;
			    if difference  > player.money {
				println!("you have to put in the rest of your chips");
				self.pot += player.money;
				cumulative_bets[i] += player.money;
				player.money = 0.0;				
				
			    } else {				
				self.pot += difference;
				cumulative_bets[i] += difference;				
				player.money -= difference;
			    }
			    num_settled += 1;
			},
			PlayerAction::Bet(new_bet) => {
			    println!("Player bets {}!", new_bet);			    			    			    			    
			    let difference = new_bet - player_cumulative;			    
			    self.pot += difference;
			    player.money -= difference;
			    street_bet = new_bet;
			    cumulative_bets[i] = new_bet;
			    num_settled = 1; // since we just bet more, we are the only settled player
			}
		    }
		}
		println!("num_active = {}, num_settled = {}", self.num_active, num_settled);
		if self.num_active == 1 {
		    println!("Only one active player left so lets break the steet loop");
		    break 'street;
		}
		if num_settled == self.num_active {
		    // every active player is ready to move onto the next street
		    println!("everyone is ready to go to the next street! num_settled = {}", num_settled);
		    break 'street;
		}
		
	    }
	}
    }
    

    fn get_action_from_user(player: &Player) -> PlayerAction {
	// will need UI here
	// for now do a random action
	
	let num = rand::thread_rng().gen_range(0..100);
	match num {
	    0..=10 => PlayerAction::Fold,
	    11..=39 => PlayerAction::Check,
	    40..=70 => {
		let amount = rand::thread_rng().gen_range(1..player.money as u32);		
		PlayerAction::Bet(amount as f64) // bet random amount
	    },
	    _ => PlayerAction::Call,
	}
    }

    fn get_and_validate_action(player: &Player, street_bet: f64, player_cumulative: f64 ) -> PlayerAction {
	// if it isnt valid based on the current bet and the amount the player has already contributed, then it loops
	let mut action;
	'valid_check: loop {
	    action = GameHand::get_action_from_user(player);
	    match action {
		PlayerAction::Fold => {		
		    //println!("Player folds!");
		    if street_bet <= player_cumulative {
			// if the player has put in enough then no sense folding
			//println!("you said fold but we will let you check!");
			action = PlayerAction::Check;
		    }
		    break 'valid_check;						    
		}
		PlayerAction::Check => {
		    //println!("Player checks!");				
		    if street_bet > player_cumulative {
			// if the current bet is higher than this players bet
			//println!("invalid action!");
			continue;
		    }
		    break 'valid_check;				
		},
		PlayerAction::Call => {
		    //println!("Player calls!");				
		    if street_bet <= player_cumulative {
			//println!("invalid action!");
			continue;
		    }
		    break 'valid_check;				
		},
		PlayerAction::Bet(new_bet) => {
		    //println!("Player bets {}!", new_bet);								
		    if street_bet < player_cumulative {
			// will this case happen?
			println!("this should not happen!");
			continue;
		    }
		    if new_bet - player_cumulative  > player.money {
			//println!("you cannot bet more than you have!");
			continue;
		    }
		    if new_bet <= street_bet {
			//println!("the new bet has to be larger than the current bet!");
			continue;
		    }
		    break 'valid_check;				
		}
	    }
	}
	action
    }
}

struct Game {
    deck: Deck,
    players: Vec<Player>,
    button_idx: usize, // index of the player with the button
    small_blind: u32,
    big_blind: u32,
}


impl Game {
    fn new() -> Self {	
	Game {
	    deck: Deck::new(),
	    players: Vec::<Player>::with_capacity(9),
	    small_blind: 4,
	    big_blind: 8,
	    button_idx: 0
	}
    }

    fn add_player(&mut self, player: Player) {
	self.players.push(player)
    }

    fn play_one_hand(&mut self) {
	let mut game_hand = GameHand::new(&mut self.deck, &mut self.players, self.button_idx
	);
	game_hand.play();	    
    }

    fn play(&mut self) {
	loop {
	    self.play_one_hand();
	    // TODO: do we need to add or remove any players?
	    
	    // TODO: what happens with the button_idx  if a player leaves
	    self.button_idx += 1; // and modulo length
	    if self.button_idx as usize >= self.players.len() {
		self.button_idx = 0;
	    }
	    
	    break; // TODO: when should the game actually end
	}
    }
}

fn main() {
    println!("Hello, world!");    
    let mut game = Game::new();
    let num_players = 2;
    for i in 0..num_players {
	let name = format!("Mr {}", i);
	game.add_player(Player::new(name));
    }
    game.play();
}
