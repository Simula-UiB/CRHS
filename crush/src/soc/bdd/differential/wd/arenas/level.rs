// #[cfg(feature = "unstable")]
// use std::collections::TryReserveError;

use ahash::AHasher;
use std::collections::BTreeMap;
use std::collections::hash_map::{Entry, Keys};
use std::collections::hash_map;
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasherDefault;
use std::iter::Filter;
use std::iter::FromIterator;
use std::vec::IntoIter;

use crate::AHashMap;
use crate::soc::{Id as NodeId, Id};
use crate::soc::bdd::differential::Depth;
use crate::soc::bdd::differential::wd::distribution::{Node2NodeDistribution, NWDistribution};
use crate::soc::bdd::differential::wd::NcWDistribution;

use super::super::PathCount;

// ************************************* Contents **************************************************
// struct WDLevel<W>
// - impl<W: NWDistribution> WDLevel<W>
// - impl<W: NcWDistribution> WDLevel<W>
// - impl<W: Node2NodeDistribution> WDLevel<W>
// - impl<W> FromIterator<(NodeId, W)> for WDLevel<W>
// - impl<'a, W> FromIterator<(&'a NodeId, W)> for WDLevel<W>
// - impl<W: NWDistribution> fmt::Debug for WDLevel<W>
// *********************************** Contents End ************************************************

#[derive(Clone)]
pub struct WDLevel<W> {
    /// Depth of level, optional to set.
    depth: Option<Depth>,
    dists: HashMap<NodeId, W, BuildHasherDefault<AHasher>>,
}

impl<W> WDLevel<W> {
    pub fn new(depth: Option<Depth>) -> Self {
        Self {
            depth,
            dists: HashMap::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut dists = HashMap::default();
        dists.reserve(capacity);
        Self {
            depth: None,
            dists,
        }
    }

    pub fn set_depth(&mut self, depth: Depth) {
        self.depth = Some(depth);
    }

    pub fn depth(&self) -> Option<usize> {
        self.depth
    }

    pub fn insert(&mut self, node: NodeId, distribution: W) {
        self.dists.insert(node, distribution);
    }

    pub fn entry(&mut self, node: NodeId) -> Entry<NodeId, W> {
        self.dists.entry(node)
    }

    pub fn get(&self, node_id: &NodeId) -> Option<&W> {
        self.dists.get(node_id)
    }

    pub fn iter(&self) -> Iter<'_, Id, W> {
        self.dists.iter()
    }

    pub fn into_iter(self) -> hash_map::IntoIter<Id, W> {
        self.dists.into_iter()
    }

    pub fn keys(&self) -> Keys<'_, Id, W> {
        self.dists.keys()
    }

    pub fn len(&self) -> usize {
        self.dists.len()
    }

    /// Returns the 'width' of the level, that is, the number of nodes registered.
    pub fn width(&self) -> usize {
        self.dists.len()
    }

    /// Clears the Level, removing all NodeId-W pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.depth = None;
        self.dists.clear();
    }

    // #[cfg(feature = "unstable")]
    // pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
    //     self.dists.try_reserve(additional)
    // }
}

impl<W: NWDistribution> WDLevel<W> {
    /// Returns the highest 'lew' (lowest existing weight) from all the weight distributions in the
    /// level.
    ///
    /// Panics if the level is empty!
    pub fn highest_lew(&self) -> u32 {
        self.dists.iter()
            .map(|(_, distr)| distr.lowest_existing_weight())
            .max().expect("The level is empty!")
    }

    /// A sorted set of the LEWs (Lowest Existing Weight) in the level, and the count of how many
    /// nodes have that LEW.
    pub fn count_lews(&self) -> BTreeMap<u32, i32> {
        let mut map = BTreeMap::new();
        self.dists.iter()
            .map(|(_, distr)| distr.lowest_existing_weight())
            .for_each(|lew| {
                let count = map.entry(lew).or_insert(0);
                *count += 1;
            });
        map
    }

    // todo remove or change into returning an iterator instead! This is a potential memory hog!
    pub fn nodes_with_lew(&self, lew: u32) -> Vec<NodeId> {
        self.dists.iter()
            .map(|(id, distr)|(id, distr.lowest_existing_weight()))
            .filter(|(_, nlew)| *nlew == lew)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

impl<W: NWDistribution> WDLevel<W> {
    pub fn lew(&self) -> (u32, Vec<NodeId>) {
        let mut map = BTreeMap::new();
        for (id, lew) in self.dists.iter()
            .map(|(id, dist)| (id, dist.lowest_existing_weight()))
        {
            let ids = map.entry(lew).or_insert(Vec::new());
            ids.push(id.clone());
        }
        map.into_iter().next().unwrap()
    }

    /// Returns all dists whose lew is the level nt-lew.
    /// This means that nodes whose lew is the trivial lew but which also the level nt-lew present
    /// are *not* included. If they are desired, then currently you must filter for them yourself.
    /// (NOTE: under normal differential/linear hull searches, a node will have its lew be either
    /// the trivial lew OR the nt_lew, but not both. So this may be an issue only for other use cases).
    pub fn nt_lew(&self) -> Option<(u32, Vec<NodeId>)> {
        let mut map = BTreeMap::new();

        for (id, lew) in self.dists.iter()
            .map(|(id, dist)| (id, dist.lowest_existing_non_trivial_weight()))
        {
            if lew.is_some() {
                let ids = map.entry(lew.unwrap()).or_insert(Vec::new());
                ids.push(id.clone());
            }
        }
        if map.is_empty() {
            None
        } else {
            Some(map.into_iter().next().unwrap())
        }
    }

    pub fn existing_weights(&self) -> BTreeMap<u32, Vec<NodeId>> {
        let mut map = BTreeMap::new();
        for (id, ws) in self.dists.iter()
            .map(|(id, dist)| (id, dist.existing_weights()))
        {
            for w in ws {
                let ids = map.entry(w).or_insert(Vec::new());
                ids.push(id.clone());
            }
        }
        map
    }
}

impl<W: NcWDistribution> WDLevel<W> {

    pub fn paths_for_weight(&self) {
        todo!()
    }
}

impl<W: Node2NodeDistribution> WDLevel<W> {

    /// Iterate over all connections which does not contain the trivial lew. Each connection may
    /// have different lews, meaning that also connections with their lew being different than the
    /// level lew are present
    pub fn iter_nt_lew_connections(&self) -> Filter<Iter<'_, Id, W>, fn(&(&'_ Id, &'_ W)) -> bool> {
        self.iter().filter(|(_, dist)| !dist.contains_trivial_lew())
    }

    // the name is slightly misvisende, as it returns the id and distribution of all level nt-lew
    // nodes, a connection would be one start node to one end node, not one start node to potentially
    // multiple end nodes, as it does now.
    /// Returns an iterator over start_ids and their distributions, whose lew is the level nt-lew.
    ///
    pub fn iter_level_nt_lew_connections(&self) -> IntoIter<(&Id, &W)> {
        // This is awkward way to do this, but it gets the job done...

        // get level nt-lew
        let maybe_nt = self.nt_lew();
        let g: Vec<(&Id, &W)> =
            if maybe_nt.is_some() {
                // Get the starting id's which contains the level nt-lew
                let (_, mut start_ids) = maybe_nt.unwrap();
                // Not sure if having it sorted speeds up any searching, but I hope it does.
                start_ids.sort();

                // Filter away the rest
                self.dists.iter().
                    filter(|(start_id, _)| start_ids.contains(start_id))
                    .collect()

            } else {
                // We don't have a level nt lew, b/c all distributions in level only have the trivial lew/
                // Therefore, we return, in effect, None
                vec![]
            };
        g.into_iter()
    }

    /// For each connection from self to end node, return a mapping from
    /// end-node to existing weights to count
    pub fn existing_weights_with_paths_per_connection(&self)
        -> AHashMap<NodeId, AHashMap<NodeId, BTreeMap<u32, &PathCount>>> {

        let mut map = HashMap::default();
        for (start_id, w_map) in self.dists.iter()
            .map(|(start_id, dist)| (start_id.clone(), dist.existing_weights_with_paths_per_connection()))
        {
            map.insert(start_id, w_map);
        }
        map
    }


    /// Identify (one of) the connection(s) which contains the most nr of paths for the level nt-lew.
    /// Returns None if none of the connections have a nt-lew
    /// FIXME make generic, i.e. for any given weight, find a connection with max paths
    pub fn nt_lew_connection_max_paths(&self) -> Option<(u32, NodeId, NodeId, PathCount)> {
        let mut sorted = BTreeMap::new();

        for (start_id, maybe_nt) in self.dists.iter()
            .map(|(start_id, dist)| {
            // (start_id, Option<(lew, Vec<end_id>)>)
                (start_id, dist.nt_lew_and_e_ids())
            }) {

            if maybe_nt.is_some() {
                let (lew, end_ids) = maybe_nt.unwrap();
                // We have lew, start, end_ids
                // I'd like to have lew, start, end, max paths w/lew
                let e_id_lew_paths: BTreeMap<&PathCount, &NodeId> =  end_ids.iter()
                    .map(|e_id| (e_id,
                                 self.dists.get(start_id)
                                     .unwrap()
                                     .paths_for_weight_in_id(lew, e_id)
                                     .unwrap())
                    )
                    // .filter(|(_, paths)| paths.is_some())
                    .map(|(id, paths)| (paths, id))
                    .collect();
                let (paths, end_id) = e_id_lew_paths.into_iter().next().unwrap();
                // Mulig fixme
                // Now we have start, end, lew, paths
                let p = sorted.entry(lew).or_insert(BTreeMap::new());
                // (lew, (paths, (start_id, end_id)))
                p.insert(paths.clone(), (*start_id, *end_id));

            }
        }

        if sorted.is_empty() {
            return None;
        }
        // Get lowest nt lew
        let (lew, paths) = sorted.iter().next().unwrap();
        // get highest paths for that lew, and start and end node
        let (path, (start_id, end_id)) = paths.iter().last().unwrap();


        let paths = path.clone();
        Some((lew.clone(), *start_id, *end_id, paths))
    }
}

impl<W> FromIterator<(NodeId, W)> for WDLevel<W> {
    fn from_iter<I: IntoIterator<Item = (NodeId, W)>>(iter: I) -> Self {
        let mut level = HashMap::default();
        level.extend(iter);
        Self {
            depth: None,
            dists: level
        }
    }
}

impl<'a, W> FromIterator<(&'a NodeId, W)> for WDLevel<W> {
    fn from_iter<I: IntoIterator<Item = (&'a NodeId, W)>>(iter: I) -> Self {
        let mut level = HashMap::default();
        level.extend(
            iter.into_iter()
            .map(|(id, w)| (id.clone(), w))
            .collect::<HashMap<NodeId, W, BuildHasherDefault<AHasher>>>()
        );
        Self {
            depth: None,
            dists: level
        }
    }
}

impl<W: NWDistribution> fmt::Debug for WDLevel<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.depth {
            Some(depth) => writeln!(f, "{: >4}Depth {}:", "", depth)?,
            None => writeln!(f, "{: >4}Depth: N/A", "")?,
        };



        for (id, weight) in self.dists.iter() {
            let highest = *weight.existing_weights().iter().last().unwrap();
            let dist = format!("{:0>w$b}", weight.existing_weights().iter()
                .fold(0_u128,
                      |acc, i| {acc | 1 << i }
                ), w = highest as usize
            );


            // writeln!(f, "{: >8}Id: {: >11}, trail_high: {: >3}, trail_low: {: >3}. Dist len: {}, Dist: {}. Raw: {:?}",
            writeln!(f, "{: >8}Id: {: >11}, trail_high: {: >3}, trail_low: {: >3}. Dist len: {}, Dist: {}.",
                     "",
                     id,
                     highest,
                     weight.lowest_existing_weight(),
                     W::SUPPORTED_DISTRIBUTION_LEN,
                     dist,
                     // weight,
            )?;
        };
        Ok(())
    }
}