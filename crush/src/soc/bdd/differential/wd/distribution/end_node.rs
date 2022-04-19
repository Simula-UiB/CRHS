use std::collections::btree_map::Iter;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ops::{Add, AddAssign};

use crate::AHashMap;
use crate::soc::bdd::differential::wd::{NcWDistribution, NWDistribution, WDCountV2};
use crate::soc::bdd::differential::wd::distribution::Node2NodeDistribution;
use crate::soc::Id;

use super::PathCount;

#[derive(Hash, Debug, Clone, Eq, PartialEq)]
pub struct EndNodeDist<W: NcWDistribution = WDCountV2> {
    // For some reason, it claims that Hash is not satisfied when I try to use AHashMap..? TODO fixed?
    map: BTreeMap<Id, W>,
}

impl<W: NcWDistribution> EndNodeDist<W> {
    #[inline]
    pub fn iter(&self) -> Iter<'_, Id, W> {
        self.map.iter()
    }

    #[inline]
    pub fn nr_of_end_nodes(&self) -> usize {
        self.map.len()
    }
}

impl<W: NcWDistribution> NWDistribution for EndNodeDist<W> {
    const SUPPORTED_DISTRIBUTION_LEN: usize = W::SUPPORTED_DISTRIBUTION_LEN;

    #[inline]
    fn new_zeroed() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    #[inline]
    fn new_trivial(id: &Id) -> Self {
        let mut map = BTreeMap::new();
        map.insert(*id, W::new_trivial(id));
        Self {
            map,
        }
    }

    #[inline]
    fn increment_distribution(&mut self) {
        for wdc in self.map.iter_mut() {
            wdc.1.increment_distribution();
        }
    }

    #[inline]
    fn lowest_existing_weight(&self) -> u32 {
        let mut lews: Vec<u32> = self.map.iter()
            .map(|(_, dist)| (dist.lowest_existing_weight()))
            // Would be nice if I could short-circuit immediately if 0 is seen...
            .collect();

        lews.sort();
        lews[0]
    }

    #[inline]
    fn lowest_existing_non_trivial_weight(&self) -> Option<u32> {
        let mut lews: Vec<u32> = self.map.iter()
            .filter_map(|(_, dist)| dist.lowest_existing_non_trivial_weight())
            .collect();

        // Old code, in case I've misunderstood this one. TODO bugsjekk!
            // .map(|(_, dist)| (dist.lowest_existing_non_trivial_weight()))
            // .filter_map()
            // .filter(|lew| lew.is_some())
            // .map(|lew| lew.unwrap())
            // .collect();
        if lews.is_empty() {
            None
        } else {
            lews.sort();
            Some(lews[0])
        }
    }

    #[inline]
    fn contains_trivial_lew(&self) -> bool {
        for (_, dist) in self.map.iter() {
            if dist.contains_trivial_lew() == true {
                return true
            }
        }
        false
    }

    #[inline]
    fn existing_weights(&self) -> BTreeSet<u32> {
        self.map.iter()
            .map(|(_, dist)| dist.existing_weights())
            .fold(BTreeSet::new(),
            |mut acc, dist| { acc.extend(dist); acc }
            )
    }
}

impl<W: NcWDistribution> Node2NodeDistribution  for EndNodeDist<W> {
    type W = W;

    #[inline]
    fn nt_lew_and_e_ids(&self) -> Option<(u32, Vec<Id>)> {
        let mut lew_id = BTreeMap::new();

        for (id, dist) in self.map.iter() {
            let lew = dist.lowest_existing_weight();
            let ids = lew_id.entry(lew).or_insert(Vec::new());
            ids.push(id.clone());
        }

        let mut iter = lew_id.iter();
        // Getting lew, it may or may not be the trivial path.
        let maybe_trivial = iter.next()
            .expect("A weight distribution should never be completely empty!");


        return if maybe_trivial.0 == &0 {
            if lew_id.len() == 1 {
                // The lew is the trivial lew, and its the only weight present. I.e. only the trivial lew
                // is present
                None
            } else {
                // The lew is trivial, but more weights are present. Return the lowest.
                let (nt_lew, e_ids) = iter.next().unwrap();
                Some((nt_lew.clone(), e_ids.clone()))
            }
        } else {
            // The lew is no-trivial, return it.
            Some((maybe_trivial.0.clone(), maybe_trivial.1.clone()))
        }

    }

    #[inline]
    fn paths_for_weight_in_id(&self, weight: u32, id: &Id) -> Option<&PathCount> {
        self.map.get(id)?.paths_for_weight(weight)
    }

    #[inline]
    fn paths_for_weight(&self, weight: u32) -> Option<AHashMap<Id, &PathCount>> {
        let hm: AHashMap<Id, &PathCount> = self.map.iter()
            .map(|(id, dist)| (id, dist.paths_for_weight(weight)))
            .filter(|(id, dist)| dist.is_some())
            .map(|(id, dist)| (id.clone(), dist.unwrap()))
            .collect();

        if hm.is_empty() {
            None
        } else {
            Some(hm)
        }
    }

    #[inline]
    fn lew_with_paths_per_connection(&self) -> AHashMap<Id, (u32, &PathCount)> {
        self.map.iter()
            .map(|(id, dist)| (id.clone(), dist.lew_with_paths()))
            .collect()
    }

    #[inline]
    fn nt_lew_with_paths_per_connection(&self) -> Option<AHashMap<Id, (u32, &PathCount)>> {
        let hm: AHashMap<Id, (u32, &PathCount)> =
            self.map.iter()
                .map(|(id, dist)| (id, dist.nt_lew_with_paths()))
                .filter(|(id, dist)| dist.is_some())
                .map(|(id, dist)| (id.clone(), dist.unwrap()) )
                .collect();

        if hm.is_empty() {
            None
        } else {
            Some(hm)
        }
    }


    #[inline]
    fn existing_weights_with_paths_per_connection(&self) -> AHashMap<Id, BTreeMap<u32, &PathCount>> {
        self.map.iter()
            .map(|(id, dist)| (id.clone(), dist.existing_weights_with_counts()))
            .collect()
    }


    #[inline]
    fn other_node(&self, other_id: &Id) -> Option<&Self::W> {
        self.map.get(other_id)
    }
}

impl<W: NcWDistribution> Add<Self> for EndNodeDist<W> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let mut res = self;
        for (id, weight) in rhs.map.iter() {
            let tot_weight = res.map.entry(*id).or_insert(W::new_zeroed());
            *tot_weight += weight.clone();
        }

        res
    }
}

impl<W: NcWDistribution> AddAssign<Self> for EndNodeDist<W> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        for (id, weight) in rhs.map.iter() {
            let tot_weight = self.map.entry(*id).or_insert(W::new_zeroed());
            *tot_weight += weight.clone();
        }
    }
}