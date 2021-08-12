use ahash::AHasher;
use num_traits::{One, Zero};
use std::cell::RefCell;
use std::collections::{btree_map, BTreeMap, BTreeSet, HashMap};
use std::collections::VecDeque;
use std::convert::From;
use std::fmt;
use std::hash::BuildHasherDefault;
use std::ops::{Add, AddAssign};
use std::ops::{Deref, DerefMut};

use crate::soc::bdd::differential::Depth;
use crate::soc::Id;

pub type PathCount = u128;
pub type NWAreaLevel = HashMap<Id, u128, BuildHasherDefault<AHasher>>;
// pub type PWCArenaLevel = HashMap<Id, PWCount, BuildHasherDefault<AHasher>>;
/// Capacity of a PathWeightCount
// (We use 128, as we expect to prune before we exceed a path w/ weight 127).
const CAPACITY: usize = 128;


/// An arena connecting node Id's, weights and levels together.
///
/// This arena is a BtreeMap wrapping a HashMap. The key for the BTreeMap is `Depth`, meaning the
/// depth a level resides at. This will yield a HashMap as a value, who in term accept an `Id` as
/// key, and a `weight` as value. The weight attribute says which trail weights pass through the
/// node corresponding to `Id`. See module level documentation for more on how weight trails are
/// recorded in `weight`.
///
/// The 'top' of an arena is always towards 'depth 0', and 'bottom' is always towards the depth of
/// a CHRS equations' sink.
///
/// This means that Id's are sorted by levels.
///
/// Depth/level, => id, weight
//TODO impl Display
#[derive(Eq, PartialEq)]
pub struct NWArena {
    pub(super) top: usize,
    pub(super) bottom: usize,
    /// Arena to introduce indirection, mapping is :> Depth : (Id : weight)
    pub(super) arena: BTreeMap<Depth, NWAreaLevel>,
    /// Cache for highest_LSBs, their respective counts and location (depths).
    pub(super) lsb_map: RefCell<Option<BTreeMap<u32, (usize, BTreeSet<Depth>)>>>,
}

/// Value keeping track of nr of paths with weight w.
/// Where NWArena uses an u128 and true/false to keep a record over the presence of paths with given
/// weight, this struct allows to also keep record of how many paths have given weight.
/// The weight is indicated by index, as with NWArena, where index 0 indicates is the LSB, i.e.
/// the presence/absence of a path of weight 0.
#[derive(Clone, Debug)]
pub struct PWCount {
    w: VecDeque<PathCount>
}

impl NWArena {

    /// Creates a new, empty NWArena, with the working area/ active area from 'top' (inclusive) to
    /// 'bottom' (exclusive).
    pub fn new(top: usize, bottom: usize) -> Self {
        assert!(top < bottom, "'Top' must be less than 'bottom'!");
        Self {
            top,
            bottom,
            arena: Default::default(),
            lsb_map: Default::default(),
        }
    }

    /// Insert a node id into the level at depth depth. If the level did have this Id present,
    /// the weight is updated, and the old weight is returned.
    pub fn insert(&mut self, id: Id, weight: u128, depth: usize) -> Option<u128> {
        let level = self.arena.entry(depth).or_insert(HashMap::default());
        level.insert(id, weight)
    }

    /// Insert a level of nodes and weights into the arena.
    /// If the arena did have this level (depth) present, the old level is replaced, and returns the
    /// old level.
    pub fn insert_level(&mut self, level: NWAreaLevel,
                        depth: usize) -> Option<NWAreaLevel> {
        // Note, missing param checks
        self.arena.insert(depth, level)
    }

    /// Returns an iterator over the levels in the arena. The levels are guaranteed to be within
    /// the range of the 'active area', but that is also the only guarantee we give.
    pub fn iter_levels(&self) -> btree_map::Iter<'_, usize, NWAreaLevel> {
        self.arena.iter()
    }

    /// Returns the node_id: weight mapping for the level at 'depth', or None if no such mapping
    /// has been made.
    /// If it is desired, use 'make_get_level'.
    pub fn get_level(&self, depth: usize) -> Option<&NWAreaLevel> {
        self.arena.get(&depth)
    }

    /// Merges first with other. If the working areas differ, then the new 'top' is
    /// min(first.top, other.top), and new 'bottom' is max(self.max, other.max).
    pub fn merge(mut first: NWArena, other: NWArena) -> NWArena {
        first.arena.extend(other.arena);
        first.top = first.top.min(other.top);
        first.bottom = first.bottom.max(other.bottom);

        first
    }


}

// LSB related functionality for NWArena
impl NWArena {

    /// Returns the highest LSB present in self
    pub fn highest_lsb_in_arena(&self) -> u32 {
        if self.lsb_map.borrow().is_none() {
            self.fill_lsb_cache();
        }

        let len = self.lsb_map.borrow().as_ref().unwrap().len();
        self.lsb_map.borrow().as_ref().unwrap().keys().skip( len - 1 ).next().cloned().unwrap()
    }

    /// Returns the LSBs found at the given level 'depth', and their respective counts.
    /// Panics if the level is not present in self.
    pub fn lsbs_in_level(&self, depth: &Depth) -> BTreeMap<u32, usize> {
        if self.lsb_map.borrow().is_none() {
            self.fill_lsb_cache();
        }

        let level = self.arena.get(depth)
            .expect(&format!("Level not part of the Arena. Given depth was: {}", depth));

        let mut lsbs: BTreeMap<u32, usize> = Default::default();
        for (_, weight) in level.iter() {
            let lsb = weight.trailing_zeros();
            let count = lsbs.entry(lsb).or_insert(0);
            *count += 1;
        }
        lsbs
    }

    /// Returns the highest LSB for the given level 'depth', and its count: (lsb, count)
    /// Panics if the level is not present in self, and if level is empty.
    pub fn highest_lsb_in_level(&self, depth: &Depth) -> (u32, usize) {
        self.lsbs_in_level(depth).iter().last().map(|(lsb, count)| (*lsb, *count) ).expect("Level is empty")
    }

    /// Returns the lowest **non-trivial** LSB for the given level 'depth',
    /// and its count: (lsb, count).
    /// Panics if the level is not present in self, and if level is empty.
    pub fn lowest_lsb_in_level(&self, depth: &Depth) -> (u32, usize) {
        self.lsbs_in_level(depth).iter()
            .filter(|(lsb, count)| **lsb != 0)
            .next()
            .map(|(lsb, count)| (*lsb, *count) ).expect("Level is empty")
    }

    /// Returns a vec with the Id's and weight off all Nodes present at level 'depth' which has
    /// a LSB matching the given lsb.
    pub fn nodes_with_lsb_at_level(&self, depth: &Depth, lsb: u32) -> Vec<(Id, u128)> {
        self.arena.get(depth).unwrap().iter()
            .filter(|(id, weight)| weight.trailing_zeros() == lsb)
            .map(|(id, weight)| (*id, *weight))
            .collect()
    }

    /// Returns the LSB of the given node.
    pub fn node_lsb(&self, depth: &Depth, node_id: &Id) -> u32 {
        self.arena.get(depth).expect(&format!("Level not found, depth: {}", depth))
            .get(node_id).expect(&format!("Node not found, Id: {}, depth: {}", node_id, depth))
            .trailing_zeros()
    }


    /// Overview of LSBs in self: the LSBs, respective counts and locations (depths).
    /// Formatted and returned as String.
    pub fn lsbs_to_string(&self) -> String {
        if self.lsb_map.borrow().is_none() {
            self.fill_lsb_cache();
        }

        let mut all = String::new();

        for (lsb, (count, depths)) in self.lsb_map.borrow().as_ref().unwrap().iter() {
            let mut s = format!("[");
            for d in depths.iter() {
                s.push_str(&format!("{}, ", d));
            }
            s.pop();
            s.pop();
            s.push_str("]");
            all.push_str(&format!("LSB: {: >4}, Count: {: >10}, Located at Depths: {}: Tot dept: {}",
                                 lsb, count, s, depths.len()));
        }
        all
    }

    /// Fill/update the highest_lsbs_cache.
    /// Panics if the lsb of the weight is "0", as we cannot have 0-weights!
    fn fill_lsb_cache(&self) {
        // (weight : (count : depths)
        let mut mapping: BTreeMap<u32, (usize, BTreeSet<usize>)> = Default::default();

        for (depth, level) in self.iter_levels() {
            for (_id, weight) in level.iter() {
                let candidate = weight.trailing_zeros();
                let (count, depths) = mapping.entry(candidate)
                    .or_insert((0, BTreeSet::new()));
                *count += 1;
                depths.insert(*depth);
                if candidate == 128 {
                    panic!("Found a node with weight 0?!\nLevel: {}. Id: {}, weight: {}", depth, _id, weight);
                }
            }
        }
        self.lsb_map.replace(Some(mapping));
    }

}

impl fmt::Debug for NWArena {
    // FIXME update to reflect changes
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Working-area range: {}..{}", self.top, self.bottom)?;
        if self.arena.is_empty() {
            writeln!(f, "NodeWeightArena is empty.")?;
        } else {
            writeln!(f, "NodeWeightArena contains {} levels.", self.arena.len())?;
        }
        // High and low is currently index -1, counting from left of the binary representation
        // of `weights`. This corresponds to the trail of same weight.
        // Ex. A high of weight '4' means that the highest trail present has weight 4.

        for (depth, map) in self.arena.iter() {
            writeln!(f, "{: >4}Level {}:", "", *depth)?;
            for (id, weight) in map.iter() {
                writeln!(f, "{: >8}Id: {: >11}, trail_high: {: >3}, trail_low: {: >3}. Weight: {:#b}.",
                         "",
                         id,
                         127 - weight.leading_zeros(),
                         weight.trailing_zeros(),
                         weight,
                )?;
            }
        }

        Ok(())
    }
}


// ================================================================================================
// ================================================================================================
// ================================================================================================

#[derive(Clone, Debug)]
pub struct PWCArenaLevel {
    level: HashMap<Id, PWCount, BuildHasherDefault<AHasher>>,
}

impl Deref for PWCArenaLevel {
    type Target = HashMap<Id, PWCount, BuildHasherDefault<AHasher>>;

    fn deref(&self) -> &Self::Target {
        &self.level
    }
}

impl DerefMut for PWCArenaLevel {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.level
    }
}

impl PWCArenaLevel {
    pub fn new_from(level: HashMap<Id, PWCount, BuildHasherDefault<AHasher>>) -> Self {
        Self {
            level,
        }
    }

    /// Returns the lowest existing weights found in the level, and their respective counts.
    pub fn lowest_existing_weights(&self) -> BTreeMap<u32, usize> {
        let mut lews: BTreeMap<u32, usize> = Default::default();
        for (_, weight_dist) in self.level.iter() {

            let lew = weight_dist.lowest_non_zero_weight();
            let count = lews.entry(lew).or_insert(0);
            *count += 1;
        }
        lews
    }
}

impl From<PWCArenaLevel> for NWAreaLevel {
    fn from(level: PWCArenaLevel) -> Self {
        level.level.into_iter()
            .map(|(id, count)| {
                (id, count.into())
            })
            .collect()
    }
}



impl PWCount {

    /// Returns a new PWCount, filled to CAPACITY with 0's. (CAPACITY is a const set in mod).
    pub fn new() -> PWCount {
        let v: Vec<PathCount> = vec![Zero::zero(); CAPACITY];
        let mut w = VecDeque::with_capacity(CAPACITY);
        w.extend(v);

        Self {
            w,
        }
    }

    /// Returns a new self with the count of the LSB (index 0) set to 1.
    pub fn fresh_trivial() -> PWCount {
        let v: Vec<PathCount> = vec![Zero::zero(); CAPACITY];
        let mut w = VecDeque::with_capacity(CAPACITY);
        w.extend(v);
        let count = w.get_mut(0).unwrap();
        *count = One::one();
        Self {
            w
        }
    }

    /// Returns a new self with the count of the index 1 set to 1.
    pub fn fresh_one() -> PWCount {
        let v: Vec<PathCount> = vec![Zero::zero(); CAPACITY];
        let mut w = VecDeque::with_capacity(CAPACITY);
        w.extend(v);
        let count = w.get_mut(1).unwrap();
        *count = One::one();
        Self {
            w
        }
    }

    /// Corresponds to left shifting the u128 of the NWArena.
    pub fn increment_indices(&mut self) {
        self.w.pop_back();
        self.w.push_front(Zero::zero());
    }

    pub fn lowest_non_zero_weight(&self) -> u32 {
        for (i, v) in self.w.iter().enumerate() {
            if v != &Zero::zero() {
                return i as u32;
            }
        }
        panic!("Found an all zero PWCount! {:?}", self);
    }

    pub fn sum_trails(&self) -> PathCount {
        let mut acc = 0;
        for count in self.w.iter() {
            acc += count;
        }
        acc
    }

    pub fn index_non_zero_trail(&self) -> Vec<usize> {
        let mut buff = Vec::new();
        for (i, v) in self.w.iter().enumerate() {
            if v != &Zero::zero() {
                buff.push(i);
            }
        }
        buff
    }
}

impl AddAssign for PWCount {
    fn add_assign(&mut self, rhs: Self) {
        for (count, rhs_count) in self.w.iter_mut().zip(rhs.w.iter()) {
            *count += rhs_count;
        }
    }
}

impl AddAssign<&Self> for PWCount {
    fn add_assign(&mut self, rhs: &Self) {
        for (count, rhs_count) in self.w.iter_mut().zip(rhs.w.iter()) {
            *count += rhs_count;
        }
    }
}

impl AddAssign<&mut Self> for PWCount {
    fn add_assign(&mut self, rhs: &mut Self) {
        for (count, rhs_count) in self.w.iter_mut().zip(rhs.w.iter()) {
            *count += rhs_count;
        }
    }
}

impl Add for PWCount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // println!("Debug add:");
        let mut w = VecDeque::with_capacity(CAPACITY);
        for (count, rhs_count) in self.w.iter().zip(rhs.w.iter()) {
            w.push_back(count + rhs_count);
        }
        Self {
            w
        }
    }
}

impl Add<&Self> for PWCount {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        let mut w = VecDeque::with_capacity(CAPACITY);
        for (count, rhs_count) in self.w.iter().zip(rhs.w.iter()) {
            w.push_back(count + rhs_count);
        }
        Self {
            w
        }
    }
}

impl From<Vec<PathCount>> for PWCount {
    fn from(vec: Vec<PathCount>) -> Self {
        let mut w = VecDeque::with_capacity(CAPACITY);
        w.extend(vec);

        Self {
            w
        }
    }
}

impl From<PWCount> for u128 {
    fn from(counts: PWCount) -> Self {
        assert!( CAPACITY <= 128, "Higher capacities than 128 is not supported!");
        let res = counts.w.iter().enumerate()
            .map(|(i, count)| {
                if count == &Zero::zero() { (i, 0_u128) }
                else { (i, 1_u128) }
            })
            .fold(0_u128, |acc, (i, is_set)| {
                acc | (is_set << i)
            });
        res
    }
}