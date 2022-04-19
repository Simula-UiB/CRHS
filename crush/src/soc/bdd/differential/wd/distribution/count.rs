use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::Debug;
use std::ops::{Add, AddAssign};

use crate::soc::bdd::differential::wd::distribution::{NcWDistribution, NWDistribution};
use crate::soc::bdd::differential::wd::PathCount;
use crate::soc::Id;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct WDCount {
    dist: VecDeque<PathCount>,
}

impl WDCount {
    pub fn contains_weight(&self, target: usize) -> bool {
        self.dist.get(target).is_some()
    }
}

impl NWDistribution for WDCount {
    const SUPPORTED_DISTRIBUTION_LEN: usize = 128;

    fn new_zeroed() -> Self {
        let mut dist = VecDeque::with_capacity(Self::SUPPORTED_DISTRIBUTION_LEN);
        dist.extend(vec![Self::zero(); Self::SUPPORTED_DISTRIBUTION_LEN]);

        Self {
            dist,
        }
    }

    fn new_trivial(_: &Id) -> Self {
        let mut dist = VecDeque::with_capacity(Self::SUPPORTED_DISTRIBUTION_LEN);
        dist.extend(vec![Self::zero(); Self::SUPPORTED_DISTRIBUTION_LEN]);
        let count = dist.get_mut(0).unwrap();
        *count = Self::one();
        Self {
            dist
        }
    }

    fn increment_distribution(&mut self) {
        self.dist.pop_back();
        self.dist.push_front(Self::zero());
    }

    fn lowest_existing_weight(&self) -> u32 {
        for (i, v) in self.dist.iter().enumerate() {
            if v != &Self::zero() {
                return i as u32;
            }
        }
        panic!("Found WDCount with no paths! {:?}", self);
    }

    fn lowest_existing_non_trivial_weight(&self) -> Option<u32> {
        for (i, v) in self.dist.iter().enumerate().skip(1) {
            if v != &Self::zero() {
                return Some(i as u32);
            }
        }
        None
    }

    fn contains_trivial_lew(&self) -> bool {
        self.dist[0] != Self::zero()
    }

    fn existing_weights(&self) -> BTreeSet<u32> {
        let zero = Self::zero();
        let mut buff = BTreeSet::new();
        for (i, v) in self.dist.iter().enumerate() {
            if v != &zero {
                buff.insert(i as u32);
            }
        }
        debug_assert!(!buff.is_empty(), "A weight distribution should never be completely empty!");
        buff
    }
}

impl NcWDistribution for WDCount {
    fn paths_for_weight(&self, weight: u32) -> Option<&PathCount> {
        // TODO consider panic on None?
        self.dist.get(weight as usize)
    }

    fn lew_with_paths(&self) -> (u32, &PathCount) {
        for (i, v) in self.dist.iter().enumerate() {
            if v != &Self::zero() {
                return (i as u32, v);
            }
        }
        panic!("Found WDCount with no paths! {:?}", self);
    }

    fn nt_lew_with_paths(&self) -> Option<(u32, &PathCount)> {
        for (i, v) in self.dist.iter().enumerate().skip(1) {
            if v != &Self::zero() {
                return Some((i as u32, v));
            }
        }
        None
    }

    fn existing_weights_with_counts(&self) -> BTreeMap<u32, &PathCount> {
        let mut buff = BTreeMap::new();
        for (i, v) in self.dist.iter().enumerate() {
            if v != &Self::zero() {
                buff.insert(i as u32, v);
            }
        }
        debug_assert!(!buff.is_empty(), "A weight distribution should never be completely empty!");
        buff
    }

    fn total_number_of_paths_overflowing(&self) -> (usize, bool) {
        self.dist.iter()
            .map(|v| *v as usize)
            .fold((0, false),
                  |(sum, overflow), u| sum.overflowing_add(u) )
    }
}

impl WDCount {
    // Consider to use lazy_static! for these two
    // A semi-abstract way of dealing with integers. Originally used BigUint, in which it made
    // much sense to make zero and one fn's. Now, if I ever decide to go full generic wrt what type
    // the PathCount can be, then these two may come in handy again.
    // Kept for now, as it compiles and work.
    #[inline(always)]
    fn zero() -> PathCount {
        0
    }

    #[inline(always)]
    fn one() -> PathCount {
        1
    }
}

impl Add<Self> for WDCount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut dist = VecDeque::with_capacity(Self::SUPPORTED_DISTRIBUTION_LEN);
        for (count, rhs_count) in self.dist.iter()
            .zip(rhs.dist.iter()) {
            dist.push_back(count + rhs_count);
        }
        debug_assert_eq!(Self::SUPPORTED_DISTRIBUTION_LEN, dist.len());
        Self {
            dist,
        }
    }
}



impl AddAssign<Self> for WDCount {
    fn add_assign(&mut self, rhs: Self) {
        for (count, rhs_count) in self.dist.iter_mut()
            .zip(rhs.dist.iter()) {
            *count += rhs_count;
        }
    }
}

impl Debug for WDCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        for (i, w) in self.dist.iter().enumerate() {
            s = format!("{}{}:{}, ", s, i, w);
        }
        s.pop();
        s.pop();
        write!(f, "{}", s)?;
        Ok(())
    }
}

