use std::collections::HashMap;
use std::ops::Deref;

use uuid::Uuid;

/// A pot keeps track of the total money, and which player (indices) contributed
/// A game hand can have multiple pots, when players go all-in, and betting continues
#[derive(Debug)]
pub struct Pot {
    money: u32,                        // total amount in this pot
    contributions: HashMap<Uuid, u32>, // which players have contributed to the pot, and how much
    // the most that any one player can put in. If a player goes all-in into a pot,
    // then the cap is the amount that player has put in
    cap: Option<u32>,
}

impl Pot {
    fn new() -> Self {
        Self {
            money: 0,
            contributions: HashMap::new(),
            cap: None,
        }
    }

    /// a public getter, since we don't want people outside the pot manager to be chaning the field
    pub fn get_money(&self) -> u32 {
	self.money
    }
    
    /// is the given player id elligible to win the pot?
    /// i.e. have they contributed to it
    pub fn is_elligible(&self, id: &Uuid) -> bool {
	self.contributions.contains_key(id)
    }
}

/// The pot manager keeps track of how many pots there are and which players
/// how contributed how much to each.
#[derive(Debug)]
pub struct PotManager {
    pots: Vec<Pot>,
}

/// this lets us call .iter() right on the PotManager itself to get access to self.pots
impl Deref for PotManager {
    type Target = Vec<Pot>;

    fn deref(&self) -> &Self::Target {
        &self.pots
    }
}
impl PotManager {
    pub fn new() -> Self {
        // the pot manager starts with a single main pot
        Self {
            pots: vec![Pot::new()],
        }
    }

    /// returns a vec of each pot.money for the all pots
    /// useful to pass to the front end
    pub fn simple_repr(&self) -> Vec<u32> {
        self.pots.iter().filter(|x| x.money > 0).map(|x| x.money).collect()
    }

    /// given a player id and an amount they need to contribute to the pot
    /// and whether this is putting them all-in), this method puts the proper
    /// amount into the proper pot(s), and possibly create and redistribute into a new side pot
    pub fn contribute(&mut self, player_id: Uuid, amount: u32, all_in: bool) {
        println!(
            "inside contribute: {:?}, {:?}, all_in={:?}",
            player_id, amount, all_in
        );
        let mut to_contribute = amount;
        // insert_pot keeps track of the index at which we want to insert a pot (index+1) to,
        // and the new cap for the pot at that index
        let mut insert_pot: Option<(usize, u32)> = None;
        for (i, pot) in self.pots.iter_mut().enumerate() {
            let so_far = pot.contributions.entry(player_id).or_insert(0);
            if let Some(cap) = pot.cap {
                println!("cap of {}", cap);
                if *so_far > cap {
                    panic!(
                        "somehow player {} put in more than the cap for \
				    the the pot at index {}",
                        player_id, i
                    );
                } else if *so_far == cap {
                    println!("we have already filled up this pot");
                    continue;
                }
                // else, we need to put more into the pot
                let remaining = cap - *so_far; // amount left before the cap
                if remaining >= to_contribute {
                    println!(
                        "the new contribution fits since {} > {}",
                        remaining, to_contribute
                    );
                    *so_far += to_contribute;
                    pot.money += to_contribute;
                    if all_in {
                        // our all-in is smaller than the previous all-in
                        println!("our all-in is smaller than the previous all-in");
                        //pot.cap = Some(pot.contributions[&player_id]);
                        insert_pot = Some((i, pot.contributions[&player_id]));
                    }
                    break;
                } else {
                    // we need to contribute to the cap, then put more in the next pot
                    println!("we need to contribute to the cap, then put more in the next pot");
                    *so_far += remaining;
                    pot.money += remaining;
                    assert!(*so_far == cap);
                    to_contribute -= remaining;
                    println!("still need to contribute {}", to_contribute)
                }
            } else {
                // there is not cap on this pot, so simply put the new money in for this player
                println!("no cap");
                *so_far += to_contribute;
                pot.money += to_contribute;
                if all_in {
                    //pot.cap = Some(pot.contributions[&player_id]);
                    insert_pot = Some((i, pot.contributions[&player_id]));
                }
                break;
            }
        }
        if let Some((index, new_cap)) = insert_pot {
            println!(
                "inserting a pot at index {} and capping the previous pot at {}",
                index + 1,
                new_cap
            );
            self.pots.insert(index + 1, Pot::new());
            self.transfer_excess(index, new_cap)
        }
    }

    /// give the index of a newly-capped pot, we move any excess contributions from the pot
    /// to the next one in the vecdeque. We also move the existing cap-differential into the pot at index+1
    /// and set the new_cap in the pot at index.
    /// This happens when a all-in happens and makes the pot at index newly-capped
    /// Note: if none of the contributions to the previous pot were higher than the new cap, then no money
    /// will need to be transfered into the new pot at index+1
    fn transfer_excess(&mut self, index: usize, new_cap: u32) {
        let prev_pot = self.pots.get_mut(index).unwrap();
        println!("prev_pot = {:?}", prev_pot);
        let mut transfers = HashMap::<Uuid, u32>::new();
        let prev_cap_opt = prev_pot.cap; // move the previous cap to the new pot (if needed)
        prev_pot.cap = Some(new_cap);
        for (id, amount) in prev_pot.contributions.iter_mut() {
            //let b: bool = id;
            if *amount > new_cap {
                // we need to move the excess above the cap of the pot to the new pot
                let excess = *amount - new_cap;
                transfers.insert(*id, excess);
                *amount = new_cap;
                prev_pot.money -= excess;
            }
        }
        println!("after taking = {:?}", prev_pot);
        println!("transfers = {:?}", transfers);
        let mut new_pot = self.pots.get_mut(index + 1).unwrap();
        new_pot.money = transfers.values().sum();
        new_pot.contributions = transfers;

        if let Some(prev_cap) = prev_cap_opt {
            // if the old pot had a cap already, then the new pot is capped at the difference.
            // e.g. if someone was all-in with 750, then someone calls to go all-in with 500,
            // the the prev_pot is NOW capped at 500, and the next pot is capped at 250
            new_pot.cap = Some(prev_cap - new_cap);
        }
    }    
    
}
