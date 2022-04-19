use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Add, AddAssign};

use crate::AHashMap;
use crate::soc::Id;

use super::PathCount;

pub mod count;
pub mod end_node;
pub mod presence;
pub mod dist_factories;
pub mod count_v2;

/// NodeWeightDistribution:
/// The weight distribution for a single node
///
/// A 'present' weight is a weight with at least one path present in the distribution. That means
/// that at least one path of that weight is present in the Node this distribution belongs to.
///
///
/// IMPORTANT: The trivial path is expected to have weight 0.
pub trait NWDistribution
    where Self: Add<Output = Self> + AddAssign + Clone + Debug + Hash {

    /// The maximum number of weights supported. I.e. this is the maximum number of active S-boxes
    /// the implementor can keep track of.
    const SUPPORTED_DISTRIBUTION_LEN: usize;

    /// Return a new instance of self, where all weights are zero.
    fn new_zeroed() -> Self;

    /// Return a new instance of self, where *only* the *trivial* path is present.
    /// Argument 'id' is expected to be the id of the node at the end of the path.
    /// This enables a path to have both a start node_id and end node_id.
    fn new_trivial(id: &Id) -> Self;

    /// We've passed a one edge, and need to update the distribution accordingly. Increment the
    /// weights by one.
    fn increment_distribution(&mut self);

    /// Return the weight of the lowest present weight.
    /// This is also know as the "lew" of a node.
    fn lowest_existing_weight(&self) -> u32;

    /// Returns the weight of the lowest existing non-trivial weight (nt-lew) of the node
    /// distribution.
    /// Returns None if only the trivial lew is present in the distribution.
    fn lowest_existing_non_trivial_weight(&self) -> Option<u32>;

    fn contains_trivial_lew(&self) -> bool;

    /// Returns a sorted set of all present weights in the distribution.
    fn existing_weights(&self) -> BTreeSet<u32>;


}

/// NodeCountedWeightDistribution:
/// The paths in the distribution has some number associated with them
pub trait NcWDistribution
    where Self: NWDistribution {

    /// For the given weight, returns the associated count. In terms of a path distribution, this
    /// 'associated count' usually means the number of paths with the given weight reachable
    /// from 'self'. (See discussion on 'number of paths' in ?? ).
    fn paths_for_weight(&self, weight: u32) -> Option<&PathCount>;

    /// Returns the weight of the 'lowest existing weight' (lew) of the node
    /// distribution, along with the associated count.
    fn lew_with_paths(&self) -> (u32, &PathCount);

    /// Returns the weight of the 'lowest existing non-trivial weight' (nt-lew) of the node
    /// distribution, along with the associated count.
    /// Returns None if only the trivial lew is present in the distribution.
    fn nt_lew_with_paths(&self) -> Option<(u32, &PathCount)>;

    /// Returns a mapping between the existing weights in the distribution, and their respective
    /// associated counts.
    /// That means that any weight missing from this map should have '0' as their respective
    /// associated count.
    fn existing_weights_with_counts(&self) -> BTreeMap<u32, &PathCount>;

    /// Returns the total number of paths present in the distribution, across all weights.
    /// Returns a tuple of the addition along with a boolean indicating whether an arithmetic
    /// overflow would occur. If an overflow would have occurred then the wrapped value is returned.
    fn total_number_of_paths_overflowing(&self) -> (usize, bool);

}

pub trait Node2NodeDistribution
    where Self: NWDistribution
{
    type W: NWDistribution;

    ///
    fn nt_lew_and_e_ids(&self) -> Option<(u32, Vec<Id>)>;

    /// Returns the corresponding count iff id is in self, and weight in id is present.
    fn paths_for_weight_in_id(&self, weight: u32, id: &Id) -> Option<&PathCount>;

    /// For the given weight, return any end_node the count corresponding to weight iff
    /// the weight is present, or None otherwise.
    fn paths_for_weight(&self, weight: u32) -> Option<AHashMap<Id, &PathCount>>;

    /// For each connection from self to end node, return the lew and its count
    /// The keys in the HashMap are the Id's of the end point of the connection.
    // TODO make return value into a NonNullPathCount
    fn lew_with_paths_per_connection(&self) -> AHashMap<Id, (u32, &PathCount)>;

    /// For each connection from self to end node, return the nt-lew and its count, or None if
    /// only the trivial lew is in the distribution.
    fn nt_lew_with_paths_per_connection(&self) -> Option<AHashMap<Id, (u32, &PathCount)>>;

    /// For each connection from self to end node, return a mapping from
    /// end-node to existing weights to count
    fn existing_weights_with_paths_per_connection(&self) -> AHashMap<Id, BTreeMap<u32, &PathCount>>;

    /// If 'other_id' is an end point (start/end node, depending on viewpoint) for this
    /// Node2NodeDistribution, then this fn will return the distribution for the paths connection
    /// self to other node.
    ///
    /// Otherwise None is returned.
    fn other_node(&self, other_id: &Id ) -> Option<&Self::W>;
}
