use rand::Rng;
use std::collections::VecDeque;
use std::ops::Range;

/// This RngGenerator trait is useful for testing probabilistic
/// behaviour by pre-defining the expected randomness
pub trait RngGenerator<T>
where
    T : rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd {
    fn gen_range(&mut self, range: Range<T>) -> T;
}

#[derive(Debug)]
pub struct FakeRandom<T> {
    numbers: VecDeque<T>
}

impl <T>RngGenerator<T> for FakeRandom<T> 
    where
    T : rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd
{
    fn gen_range(&mut self, range: Range<T>) -> T {
	if self.numbers.is_empty() {
	    // No pre-defined numbers, so make a truly random one
	    rand::thread_rng().gen_range(range)
	} else {
	    self.numbers.pop_front().unwrap()
	}
    }
}

#[derive(Debug)]
pub struct TrueRandom<T> {
    phantom: std::marker::PhantomData<T> // don't actually need it
}

impl <T>TrueRandom<T>
    where
    T : rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd
{    
    pub fn new() -> Self {
	std::marker::PhantomData
    }
}

impl <T>RngGenerator<T> for TrueRandom<T> 
    where
    T : rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd
{
    fn gen_range(&mut self, range: Range<T>) -> T {
	// just a real random call
	rand::thread_rng().gen_range(range)
    }
}
