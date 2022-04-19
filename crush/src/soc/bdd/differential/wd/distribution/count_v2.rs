use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::ops::{Add, AddAssign};

use crate::soc::bdd::differential::wd::{NcWDistribution, NWDistribution, PathCount};
use crate::soc::Id;

/// The weight of a Path. TODO consider to move to super.
type PathWeight = u8;

/// Distribution which keeps track of what weight is seen, as well as how many Paths have this
/// count.
/// This is intended to be a replacement for WDCount, with the goal of significantly reduce memory
/// consumption. This reduction is expected to come with a run-time performance cost. We'll see.
#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub struct WDCountV2 {
    // BTreeMap to allow for Hashing, Option to allow for dist increment
    // FIXME is there a way to get rid of especially the Option? Use unsafe?
    // Obs, some features now rely on the BTreeMap being sorted. Needs to change if BTreeMap is replaced!
    dist: Option<BTreeMap<PathWeight, PathCount>>,
}

impl NWDistribution for WDCountV2 {
    const SUPPORTED_DISTRIBUTION_LEN: usize = PathWeight::MAX as usize;

    #[inline]
    fn new_zeroed() -> Self {
        Self {
            dist: Some(BTreeMap::new()),
        }
    }

    /// Return a new instance of self, where *only* the *trivial* path is present.
    /// Since we're also a NcWDist, that means that `PathWeight 0` will have `PathCount` set to `1`.
    #[inline]
    fn new_trivial(_: &Id) -> Self {
        Self {
            dist: Some([(0,1)].iter().cloned().collect()),
        }
    }

    #[inline]
    fn increment_distribution(&mut self) {
        //FIXME seems very inefficient...
            self.dist = Some(self.dist.take().unwrap()
                .into_iter()
                // Filtering away keys which are too large ensures safe unchecked addition
                .filter(|kv| kv.0 < Self::SUPPORTED_DISTRIBUTION_LEN as PathWeight)
                .map(|(key, value)| {
                    (key + 1, value)
                })
                .collect());
    }

    /// Return the weight of the lowest present weight.
    /// This is also know as the "lew" of a node.
    /// Panics if self is empty (no weights is present).
    #[inline]
    fn lowest_existing_weight(&self) -> u32 {
        self.dist.as_ref().unwrap().keys()
            .next().unwrap()
            .clone()
            .into()
    }

    #[inline]
    fn lowest_existing_non_trivial_weight(&self) -> Option<u32> {
        let mut keys = self.dist.as_ref().unwrap().keys();
        let candidate = keys.next()?;
        return if *candidate != 0 {
            Some(candidate.clone().into())
        } else {
            keys.next()
                .map_or(None, |v| Some((*v).into()))
        }
    }

    #[inline]
    fn contains_trivial_lew(&self) -> bool {
        let candidate = self.dist.as_ref().unwrap().keys().next();
        if candidate.is_none() {
            return false
        } else if *candidate.unwrap() == 0 {
            return true
        }
        false
    }

    #[inline]
    fn existing_weights(&self) -> BTreeSet<u32> {
        self.dist.as_ref().unwrap().keys().map(|v| (*v).into()).collect()
    }
}

impl NcWDistribution for WDCountV2 {
    #[inline]
    fn paths_for_weight(&self, weight: u32) -> Option<&PathCount> {
        let weight = weight.try_into();
        if weight.is_err() {
            return None
        }
        self.dist.as_ref().unwrap().get(&weight.unwrap())
    }

    #[inline]
    fn lew_with_paths(&self) -> (u32, &PathCount) {
        let (key, value) = self.dist.as_ref().unwrap().iter().next().unwrap();
        ((*key).into(), value.into())
    }

    #[inline]
    fn nt_lew_with_paths(&self) -> Option<(u32, &PathCount)> {
        let mut entries = self.dist.as_ref().unwrap().iter();
        let candidate = entries.next()?;
        return if *candidate.0 == 0 {
            Some( ( (*candidate.0).into(), candidate.1.into() ) )
        } else {
            entries.next()
                .map_or(None, |v| Some( ( (*v.0).into(), v.1.into() ) ) )
        }
    }

    #[inline]
    fn existing_weights_with_counts(&self) -> BTreeMap<u32, &PathCount> {
        self.dist.as_ref().unwrap().iter()
            .map(|(key, value)| (u32::from(*key), value.into()) )
            .collect()
    }

    /// Returns the total number of paths present in the distribution, across all weights.
    /// Returns a tuple of the addition along with a boolean indicating whether an arithmetic
    /// overflow would occur. If an overflow would have occurred then the wrapped value is returned.
    fn total_number_of_paths_overflowing(&self) -> (usize, bool) {
        self.dist.as_ref()
            .unwrap()
            .values()
            .map(|v| *v as usize)
            .fold((0, false),
            |(sum, overflow), u| sum.overflowing_add(u) )
    }
}


impl Add<Self> for WDCountV2 {
    type Output = Self;

    #[inline]
    fn add(mut self, rhs: Self) -> Self::Output {
        let mut res = self.dist.take().unwrap();
        for (rhs_weight, rhs_count) in rhs.dist.as_ref().unwrap().iter() {
            let res_count = res.entry(*rhs_weight).or_insert(0);
            *res_count += rhs_count;
        }
        Self {
            dist: Some(res),
        }
    }
}

impl AddAssign<Self> for WDCountV2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        let dist = self.dist.as_mut().unwrap();

        for (rhs_weight, rhs_count) in rhs.dist.as_ref().unwrap().iter() {
            let self_count = dist.entry(*rhs_weight).or_insert(0);
            *self_count += rhs_count;
        }
    }
}


#[cfg(test)]
mod test {
    use crate::soc::bdd::differential::wd::{NcWDistribution, NWDistribution};
    use crate::soc::bdd::differential::wd::distribution::count_v2::WDCountV2;

    #[test]
    fn test_increment() {
        let expected = WDCountV2 {
            dist: Some([(1, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        let mut actual = WDCountV2 {
            dist: Some([(0, 1), (1, 52), (2, 20), (3, 42), (4, 15), (150, 20000)].iter().cloned().collect()),
        };
        actual.increment_distribution();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_nt_lew() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        assert_eq!(dist.lowest_existing_non_trivial_weight(), Some(2));
        println!("Passed first assert");

        let dist = WDCountV2 {
            dist: Some([(1, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        assert_eq!(dist.lowest_existing_non_trivial_weight(), Some(1));
        println!("Passed second assert");

        let dist = WDCountV2 {
            dist: Some([(0, 1)].iter().cloned().collect()),
        };
        assert_eq!(dist.lowest_existing_non_trivial_weight(), None);
    }

    #[test]
    fn test_contains_trivial_lew() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        assert_eq!(dist.contains_trivial_lew(), true);
        println!("Passed first assert");

        let dist = WDCountV2 {
            dist: Some([(1, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        assert_eq!(dist.contains_trivial_lew(), false);
        println!("Passed second assert");

        let dist = WDCountV2 {
            dist: Some([(0, 1)].iter().cloned().collect()),
        };
        assert_eq!(dist.contains_trivial_lew(), true);
        println!("Passed third assert");

        let dist = WDCountV2 {
            dist: Some([(15, 4)].iter().cloned().collect()),
        };
        assert_eq!(dist.contains_trivial_lew(), false);
    }

    #[test]
    fn test_existing_weights() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        assert_eq!(dist.existing_weights(), [0,2,3,4,5,151].iter().cloned().collect());
    }

    #[test]
    fn test_paths_for_weight() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };
        assert_eq!(dist.paths_for_weight(0), Some(&1));
        println!("Passed first assert");
        assert_eq!(dist.paths_for_weight(2), Some(&52));
        println!("Passed second assert");
        assert_eq!(dist.paths_for_weight(151), Some(&20000));
        println!("Passed third assert");
        assert_eq!(dist.paths_for_weight(1), None);
    }

    #[test]
    fn test_sum_weights() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20_000)].iter().cloned().collect()),
        };
        let actual = dist.total_number_of_paths_overflowing();
        let expected = 20_130;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };

        let dist2 = WDCountV2 {
            dist: Some([(1, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };

        let expected = WDCountV2 {
            dist: Some([(0, 1), (1, 1), (2, 104), (3, 40), (4, 84), (5, 30), (151, 40000)].iter().cloned().collect()),
        };

        let actual = dist + dist2;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_add_assign() {
        let dist = WDCountV2 {
            dist: Some([(0, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };

        let mut actual = WDCountV2 {
            dist: Some([(1, 1), (2, 52), (3, 20), (4, 42), (5, 15), (151, 20000)].iter().cloned().collect()),
        };

        let expected = WDCountV2 {
            dist: Some([(0, 1), (1, 1), (2, 104), (3, 40), (4, 84), (5, 30), (151, 40000)].iter().cloned().collect()),
        };

        actual += dist;
        assert_eq!(actual, expected);
    }

}