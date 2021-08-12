use core::num::NonZeroUsize;
use core::slice::Iter;
use rayon::prelude::*;
use std::vec::IntoIter;

use crate::soc::bdd::Bdd;
use crate::soc::Id;

/// DepFinder (Dependency Finder) finds all nodes connected with `root` at `step` levels below
/// `root`. A node is defined as connected with `root` if there is a path from `root` to the node.
/// `Root` is thought of as having a *dependency* on all nodes connected to it.
/// Note that `root` is the node we start the search in, *not* necessarily the root of self.
///
/// Alongside each node, a bool is given. If the bool is true, then at least one 1-edge is part
/// of the path connecting `root` and that node. Otherwise, all the edges of the path is 0-edges,
/// and a false is given.
#[derive(Debug)]
pub struct DepBoolFinder {
    /// `Depth` in the Shard which `root` is located at.
    depth: usize,
    /// Id's of all the nodes `connected` with `root` at `step` levels below `depth` (`depth` = level
    /// which `root` is at). Second element is a bool which signals whether or not at least one
    /// `1-edge` is part of the path connecting `root` and `connected`.
    dependencies: Vec<(Id, bool)>,
}

impl DepBoolFinder {

    /// Creates a new DepFinder.
    /// This is function is eager, meaning that it will identify all the relevant nodes, as defined
    /// in the struct declaration, before returning the new DepFinder.
    ///
    /// `depth` is the depth in the `Shard` which `root` is located at.
    /// `step` tells us how many levels below `root` we wish to traverse to.
    pub fn new(root: Id, depth: usize, step: NonZeroUsize, shard: &Bdd) -> Self {
        // Current version uses a modified Breadth First "Search" algorithm (BFS) to traverse
        // the paths available from root.

        let step = step.get();
        let levels = &shard.levels[depth..depth + step];

        //BFS:
        // We need two vec's to work with, one is filled while the other is drained. (Saves memory).
        // When last level in 'levels' is "drained", return the 'fill' vec.

        // Called 'zero' and 'one', to make it easy to remember which is which w.r.t. index % 2.
        let mut zero = vec![(root, false)];
        let mut one = Vec::new();


        for (i, level) in levels.iter().enumerate() {
            // Setting up the correct vec's for fill and drain.
            let (drain, fill) = match i % 2 {
                0 => (&mut zero, &mut one),
                1 => (&mut one, &mut zero),
                _ => panic!("Somehow, an n mod 2 operation resulted in something not 0 or 1!"),
            };

            for (node_id, contains_one) in drain.drain(..) {
                // Assumes that we're working on a reduced Shard, with no "dangling edges".
                // I.e. we don't handle the None case of a node lookup.
                if let Some(node) = level.get_node(&node_id) {
                    if let Some(e0) = node.get_e0() {
                        // We traversed a 0-edge, which does not affect the value in 'contains_one'
                        fill.push((e0, contains_one));
                    };
                    if let Some(e1) = node.get_e1() {
                        // We traversed a 1-edge, thus 'contains_one' will be true, independently of what is was
                        fill.push((e1, true));
                    };
                }
                // If a node is a dead end, then nothing will be pushed. This is desired, since then,
                // if all nodes are dead ends, the returned vec will be empty, and any call to next will yield None.
            }
        }


        // Retrieve the vec which contains the relevant node id's. (Last vector that served as 'fill').
        let dependencies = match levels.len() % 2 {
            0 => zero,
            1 => one,
            _ => panic!("Somehow, an n mod 2 operation resulted in something not 0 or 1!"),
        };

        // Return Self
        Self {
            depth,
            dependencies,
        }
    }

    /// Will yield an iterator over tuples `(Id, bool)`, where `Id` is the id for a node
    /// connected with `root` at `step` levels below `root`, and `bool` is true if at least one
    /// 1-edge was traversed as part of the path connecting `root` and `Id`.
    ///
    /// See struct level comments for more info.
    pub fn iter(&self) -> Iter<'_, (Id, bool)> {
        self.dependencies.iter()
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    pub fn into_par_iter(self) -> rayon::vec::IntoIter<(Id, bool)> {
        self.dependencies.into_par_iter()
    }
}

impl Iterator for DepBoolFinder {
    type Item = (Id, bool);

    /// Will yield the next Node id which 'root' is dependent upon in order to calculate its own
    /// trail weight vector.
    /// Returns None when none is left.
    /// FIXME, isn't this more of an into_iter impl?? BUG?!
    fn next(&mut self) -> Option<Self::Item> {
        self.dependencies.pop()
    }
}




#[derive(Debug)]
pub struct DepPathFinder {
    /// `Depth` in the Shard which `root` is located at.
    depth: usize,
    root: Id,
    /// Id's of all the nodes `connected` with `root` at `step` levels below `depth` (`depth` = level
    /// which `root` is at). Second element is a Vec which contains the values of the edges
    /// traversed when moving **from** `root` **to** `connected`. Meaning that edge at index 0 is
    /// the edge out from root, and the edge at index step - 1 is the edge going in to connected.
    /// (The out-edge of connected is thus *not* part of the returned vec).
    dependencies: Vec<(Id, Vec<bool>)>,
}


impl DepPathFinder {
    pub fn new(root: Id, depth: usize, step: NonZeroUsize, shard: &Bdd) -> Self {
        // TODO migrate from Vec<bool> to Vob. Migrate to use &Id instead of Id?

        // Current version uses a modified Breadth First "Search" algorithm (BFS) to traverse
        // the paths available from root.

        let step = step.get();
        let levels = &shard.levels[depth..depth + step];

        //BFS:
        // We need two vec's to work with, one is filled while the other is drained. (Saves memory).
        // When last level in 'levels' is "drained", return the 'fill' vec.

        // Called 'zero' and 'one', to make it easy to remember which is which w.r.t. index % 2.
        let mut zero = vec![(root, vec![])];
        let mut one = Vec::new();


        for (i, level) in levels.iter().enumerate() {
            // Setting up the correct vec's for fill and drain.
            let (drain, fill) = match i % 2 {
                0 => (&mut zero, &mut one),
                1 => (&mut one, &mut zero),
                _ => panic!("Somehow, an n mod 2 operation resulted in something not 0 or 1!"),
            };

            for (node_id, mut path) in drain.drain(..) {
                // Assumes that we're working on a reduced Shard, with no "dangling edges".
                // I.e. we don't handle the None case of a node lookup.
                if let Some(node) = level.get_node(&node_id) {
                    if let Some(e0) = node.get_e0() {
                        // We traversed a 0-edge.
                        // A node may have both the 0 and the 1-edge point to the same child. This
                        // should result in two pushes to fill, one with the path so far plus '0',
                        // and the other with the path so far plus '1'. We therefore clone path here,
                        // before pushing anything to it, so that it may be reused by the 1-edge case.
                        let mut p = path.clone();
                        p.push(false);
                        fill.push((e0, p));
                    };
                    if let Some(e1) = node.get_e1() {
                        // We traversed a 1-edge.
                        path.push(true);
                        fill.push((e1, path));
                    };
                }
                // If a node is a dead end, then nothing will be pushed. This is desired, since then,
                // if all nodes are dead ends, the returned vec will be empty, and any call to next will yield None.
            }
        }


        // Retrieve the vec which contains the relevant node id's. (Last vector that served as 'fill').
        let stack = match levels.len() % 2 {
            0 => zero,
            1 => one,
            _ => panic!("Somehow, an n mod 2 operation resulted in something not 0 or 1!"),
        };

        // Return Self
        Self {
            depth,
            root,
            dependencies: stack,
        }
    }

    pub fn len(&self) -> usize {
        self.dependencies.len()
    }

    /// Will yield an iterator over tuples `(Id, Vec<bool>)`, where `Id` is the id for a node
    /// connected with `root` at `step` levels below `root`, and the Vec contains the path traversed
    /// when moving **from** `root` **to** `connected`. Meaning that edge at index 0 is
    /// the edge out from root, and the edge at index step - 1 is the edge going in to connected.
    /// (The out-edge of connected is thus *not* part of the returned vec).
    pub fn iter(&self) -> Iter<'_, (Id, Vec<bool>)> {
        self.dependencies.iter()
    }

    /// Will yield an iterator over tuples `(Id, Vec<bool>)`, where `Id` is the id for a node
    /// connected with `root` at `step` levels below `root`, and the Vec contains the path traversed
    /// when moving **from** `root` **to** `connected`. Meaning that edge at index 0 is
    /// the edge out from root, and the edge at index step - 1 is the edge going in to connected.
    /// (The out-edge of connected is thus *not* part of the returned vec).
    pub fn into_iter(self) -> IntoIter<(Id, Vec<bool>)> {
        self.dependencies.into_iter()
    }
}