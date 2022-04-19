use std::collections::BTreeSet;
use std::hash::Hash;
use std::ops::Add;
use std::ops::AddAssign;
use vob::Vob;

use crate::soc::bdd::differential::wd::distribution::NWDistribution;
use crate::soc::Id;

/// A simple construct which keeps track of the *presence* of weights in a weight distribution, and
/// not the count of said weights. Meaning that it can tell you if a path of weight 'w' is present
/// in the distribution, but not how many paths of weight 'w'. Nor can it tell you which specific
/// end node it is that has contributed 'w' to the distribution, but it *can* tell you which end nodes
/// the distribution is *connected* to.
///
/// This implementation relies on u128 to note the presence of paths, limiting us to path lengths
/// of 128. We recon that that should suffice, at least in terms of differential/linear cryptanalysis:
/// 127 active S-boxes should go a long way.
/// If for some reason that is not enough, we suggest replacing the u128 with a Vob.
#[derive(Clone, Hash, Debug, Eq, PartialEq)]
pub struct WDPresence{
    dist: u128,
    // Have to use BTreeSet bc HashSet gives weird compile error TODO, fixed?
    end_nodes: BTreeSet<Id>,
}

impl NWDistribution for WDPresence {
    const SUPPORTED_DISTRIBUTION_LEN: usize = 128;

    fn new_zeroed() -> Self {
        Self{
            dist: 0,
            end_nodes: BTreeSet::default(),
        }
    }

    fn new_trivial(end_id: &Id) -> Self {
        let mut end_nodes = BTreeSet::default();
        end_nodes.insert(end_id.clone());
        Self{
            dist: 1,
            end_nodes,
        }
    }

    fn increment_distribution(&mut self) {
        self.dist = self.dist << 1;
    }

    fn lowest_existing_weight(&self) -> u32 {
        self.dist.trailing_zeros()
    }

    fn lowest_existing_non_trivial_weight(&self) -> Option<u32> {
        let dist_nt = self.dist & (u128::MAX << 1);
        return if dist_nt == 0 {
        // We only had the trivial path, and it's now removed
            None
        } else {
            // We had other paths as well, and with the trivial path masked away, we can count the trailing zeros
            Some(dist_nt.trailing_zeros())
        }
    }

    fn contains_trivial_lew(&self) -> bool {
        self.dist.trailing_zeros() == 0
    }

    fn existing_weights(&self) -> BTreeSet<u32> {
        let vob: Vob = Vob::from_bytes(&self.dist.to_be_bytes())
            .iter()
            .rev()
            .collect();
        vob.iter_set_bits(..)
            .map(|u| u as u32)
            .collect()
    }
}

impl WDPresence {
    /// The Id of all end nodes reachable from this node.
    pub fn end_connections(&self) -> &BTreeSet<Id> {
        &self.end_nodes
    }
}

impl Add<Self> for WDPresence {
    type Output = Self;

    fn add(self, rhs: WDPresence) -> Self::Output {
        let dist = self.dist | rhs.dist;
        let mut end_nodes = self.end_nodes.clone();
            end_nodes.extend(rhs.end_nodes);
        Self{dist, end_nodes}
    }
}

impl AddAssign for WDPresence {
    fn add_assign(&mut self, rhs: Self) {
        self.dist |= rhs.dist;
        self.end_nodes.extend(rhs.end_nodes);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_increment() {
        let mut actual = WDPresence::new_trivial(&Id::new(5));
        actual.increment_distribution();
        let expected = WDPresence {dist: 2, end_nodes: vec![Id::new(5)].into_iter().collect()};
        assert_eq!(actual, expected);

        let mut actual = WDPresence::new_trivial(&Id::new(5));
        actual += WDPresence {dist: 3, end_nodes: vec![Id::new(2)].into_iter().collect()};
        actual.increment_distribution();
        let expected = WDPresence {dist: 6, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual, expected);

        let mut actual = WDPresence::new_zeroed();
        actual.increment_distribution();
        let expected = WDPresence {dist: 0, end_nodes: vec![].into_iter().collect()};
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_existing_weights() {
        let expected = vec![2,3].into_iter().collect();
        let actual = WDPresence {dist: 12, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.existing_weights(), expected);

        let expected = vec![2,7].into_iter().collect();
        let actual = WDPresence {dist: 132, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.existing_weights(), expected);

        let expected = vec![0, 2, 7, 10, 15].into_iter().collect();
        let actual = WDPresence {dist: 33_925, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.existing_weights(), expected);

        let expected = vec![127].into_iter().collect();
        let actual = WDPresence {dist: 1 << 127, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.existing_weights(), expected);
    }

    #[test]
    fn test_nt_lew() {
        let actual = WDPresence {dist: 132, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.lowest_existing_non_trivial_weight(), Some(2));

        let actual = WDPresence {dist: 128, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.lowest_existing_non_trivial_weight(), Some(7));

        let actual = WDPresence {dist: 129, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.lowest_existing_non_trivial_weight(), Some(7));

        let actual = WDPresence {dist: 1, end_nodes: vec![Id::new(2), Id::new(5)].into_iter().collect()};
        assert_eq!(actual.lowest_existing_non_trivial_weight(), None);
    }

    #[test]
    fn test_lew() {
        let wd = WDPresence {dist: 12, end_nodes: vec![Id::new(5)].into_iter().collect()};
        assert_eq!(wd.lowest_existing_weight(), 2);
    }
}