

//! # Differential trail search using CRHS Equations
//!
//! ## Description
//! Methods and functions for manipulating CRHS Equations with regards to Differential trail
//! analysis.
//!
//! ## Idea
//! We can create Systems of CRHS equations representing a cipher with regards to differential
//! trails. As per the standard modelling technique[ref_paper], this SoC will contain linear dependencies
//! which needs to be absorbed. As with the normal procedure, we expect the complexity (= number
//! of nodes in the SoC) to grow to unmanageable sizes. We plan to counter this by pruning the
//! SoC when we reach certain limits. These limits are (ideally) on a per computer-system basis.
//!
//! ### The pruning process
//! When the SoC reaches a pre-defined, computer dependent, size, we will halt the process of
//! absorbing linear dependencies, and prune the SoC instead. We begin the pruning process by
//! identifying the "weight" of the differential trails which pass through the nodes. All nodes
//! containing only trails with weight above a certain threshold will then be removed. This threshold
//! will be adjusted as needed.
//! We only need to identify the "weights" of nodes at certain levels, due to how the counting
//! technique works.
//! The pruning process will end when the complexity has been brought back down to a manageable
//! level (AND we've finished removing nodes for the current threshold).
//!
//! The SoC needs to uphold certain invariants, in terms of it's state, in order to be efficient/correct?
//! (These will be listed somewhere when they have been fleshed out).
//!
//! #### Node removing strategy
//!
//!
//! ## Limitations
//! Currently only supports cipher with the same sized S-boxes. (`Step` needs to be the same).
//!
//! ## Dictionary
//! Arena
//! Weight
//! prune
//!

use ahash::AHasher;
pub use dependency_finder::{DepBoolFinder, DepPathFinder};
use logging::builders::*;
pub use logging::PruneLogger;
pub use logging::records::*;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::ops::{Bound, Range, RangeBounds};
use w_arenas::{NWAreaLevel, NWArena};
use w_arenas::{PWCArenaLevel, PWCount};
pub use wide_count_prune_core::{PPFactory, StyledProgressBar};

use super::*;

pub mod wd;

mod w_arenas;
mod wide_prune_core;
mod wide_count_prune_core;
mod dependency_finder;
mod logging;
pub mod post_processing;


pub type Depth = usize;

impl Bdd {



    /// Performs the process of identifying differential trails and their weights.
    /// Returns an arena containing the Id of the "counted" nodes and the weight of the trails
    /// passing through those nodes. Not all nodes are "counted" as that is not needed. (TODO explain better).
    ///
    /// todo explain how "weight" works, and that we don't count the number of paths through nodes in
    /// this method.
    ///
    /// Trails for our purposes is one path from one edge (= top/bottom level) of the active area
    /// (= working_range) to the opposite edge. One such path will assign values to the (potentially
    /// partial) state represented in corresponding LHS's.
    ///
    /// The weight of such a path/trail is a count of how many 'active' S-boxes that's part of the
    /// path. Each S-box is represented by `step` consecutive/adjacent levels, and an S-box is
    /// considered 'active' for a given path iff that path crosses through at least one '1-edge'
    /// when passing through the levels representing the S-box.
    /// As such, there is an important invariant that must be upheld for this function to return
    /// correct results:
    ///
    /// ## Correctness Contract
    /// ***These invariants are the responsibility of the caller to uphold***. Failing to do so will
    /// probably lead to *wrong results*. (
    /// 1) The levels of `working_range` **must** be organized such that **all** levels *originating*
    ///     from the same S-box are adjacent.
    /// 2) Each S-box is represented by *exactly* `step` levels.
    /// 3) The `working_range`'s `start` can only be a *top level* of an S-box.
    /// 4) If the `working_range` `end` is:
    ///     - *exclusive*, it needs to be the level **immediately after** the
    ///     *last level of the last S-box* included in the counting.
    ///     - *inclusive*, then it needs to be the *last level of last S-box* included in the
    ///      counting.
    ///
    /// Note that no restrictions are imposed on the levels **outside* the working range.
    pub fn identify_trails_and_weights<R> (&self, working_range: R, step: usize) -> NWArena
        where
            R: RangeBounds<usize>
    {
        // Wrapper accepting RangeBounds, whereas the 'core' wants a Range
        self.identify_trails_and_weights_core(&self.process_range(working_range, step), step)
    }



    // This fn feels misplaced. If a module for post processing is ever made, consider to move it
    // there, as post processing is currently its only usage.
    /// Mapping of the highest LSB's: What depths are the various highest LSB's located at
    pub fn map_highest_lsb(&self, arena: &NWArena) -> u128 {
        // (weight : (count : depths)
        let mut mapping: BTreeMap<u128, (usize, BTreeSet<usize>)> = Default::default();

        for (depth, level) in arena.iter_levels() {
            for (_id, weight) in level.iter() {
                let candidate = weight.trailing_zeros() as u128;
                let (count, depths) = mapping.entry(candidate)
                    .or_insert((0, BTreeSet::new()));
                *count += 1;
                depths.insert(*depth);
                if candidate == 128 {
                    eprintln!("Found a node with weight 0?!");
                    eprintln!("Level: {}. Id: {}, weight: {}", depth, _id, weight);
                }
            }
        }

        for (lsb, (count, depthss)) in mapping.iter() {
            let mut s = format!("[");
            for d in depthss.iter() {
                s.push_str(&format!("{}, ", d));
            }
            s.pop();
            s.pop();
            s.push_str("]");
            println!("LSB: {: >4}, Count: {: >10}, Located at Depths: {}: Tot dept: {}", lsb, count, s, depthss.len());
        }

        // FIXME, notices that the skip probably can be replaced with 'last()', but I don't have the
        // time right now to ensure that that change indeed does not change anything.
        let lsb = mapping.keys().skip( mapping.len() - 1 ).next().cloned().unwrap();
        lsb
    }




    /// Core loop of the process of identifying differential trails and their weights.
    /// Trails for our purposes is one path from one edge (= top/bottom level) of the 'active area'
    /// (= working_range) to the opposite edge. One such path will assign values to the (potentially
    /// partial) state represented in corresponding LHS's.
    ///
    /// The weight of such a path/trail is a count of how many 'active' S-boxes that's part of the
    /// path. Each S-box is represented by `step` consecutive/adjacent levels, and an S-box is
    /// considered 'active' for a given path iff that path crosses through at least one '1-edge'
    /// when passing through the levels representing the S-box.
    /// As such, there is an important invariant that must be upheld for this function to return
    /// correct results:
    ///
    /// ## Correctness Contract
    /// **These invariants are the responsibility of the caller to uphold**. Failing to do so will
    /// probably lead to *wrong results*.
    /// 1) The levels of `working_range` **must** be organized such that **all** levels *originating*
    ///     from the same S-box are adjacent. In other words, the levels are expected to be
    ///     organized into `Cohort`s.
    /// 2) There are *exactly* `step` levels originating from any S-box. (All S-boxes contribute
    ///     with `step` levels). I.e. all Cohorts are of the same size.
    /// 3) The `working_range`'s `start` can only be a *top level* of an S-box. (The
    ///     `working_range`'s `start` can only be a `Centurion`).
    /// 4) If the `working_range` `end` is:
    ///     - *exclusive*, it needs to be the level *immediately after* the
    ///     *last level of the last S-box* included in the counting. (I.e. immediately after the
    ///     last member of the last Cohort).
    ///     - *inclusive*, then it needs to be the *last level of last S-box* included in the
    ///      counting. (AKA The last member of the last cohort).
    ///
    /// Note that no restrictions are imposed on the levels **outside* the working range.
    fn identify_trails_and_weights_core(&self, working_range: &Range<usize>, step: usize) -> NWArena {
        assert_ne!(0, step, "Step cannot be 0!");
        let nz_step = NonZeroUsize::new(step).unwrap();
        // let path = ["out_results", "In_pruning_916.dot"].iter().collect();
        // crate::soc::util::print_bdd_to_graphviz(&self, &path);

        let top = working_range.start; // Inclusive
        let bottom = working_range.end; // Exclusive

        // The part of the shard which is relevant to us, and which satisfies the module level invariants.
        let work_area = &self.levels[top..bottom];
        // Get index to "base case": Lowest level in the shard that will have weights associated with it
        let base_case_index = bottom - step;

        // Init "base case": Go `step` down, and see if an 1-edge or more was traversed, set weight accordingly
        // (We use u128, as we expect to prune before we exceed a path w/ weight 127).
        let bc_arena: HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>> =
            // "- top" is an offset, since work_area is a slice of all Levels in self.
            work_area[base_case_index - top].get_nodes().keys()
                .map(|id| (id, DepBoolFinder::new(*id, base_case_index, nz_step,
                                                  self)))
                .map(|(id, deps)| {
                    // OR together the correct weights for each path
                    let trails = deps.iter()
                        //Did we pass at least one 1-edge on the way to id?
                        .map(|(_id, edge)| if *edge {2} else {1} )
                        .fold(0, |acc, weight| { acc | weight });
                    (*id, trails)
                }).collect();


        let mut arena =  NWArena::new(top, bottom);
        let _ = arena.insert_level(bc_arena, base_case_index);
        // Are we done?
        if bottom - top == step {
            return arena;
        }

        // Step through the working area (bottom up), calculating the weights for the nodes
        // at `step` intervals.
        work_area.iter()
            .rev()
            .skip(step*2 -1)
            .step_by(step).enumerate()
            //Must be a better way to do this...
            .for_each(|(i, level)| {
                let depth = base_case_index - step*(i+1);
                let next_level = base_case_index - step*i;
                for id in level.get_nodes().keys() {
                    let trails = self.calculate_trail_weights_for_node(&id, depth,
                                                                       nz_step,
                                                                       &arena.arena.get(&next_level).unwrap());
                    arena.insert(*id, trails, depth);
                }
            });

        arena
    }


    /// Returns an ArenaLevel for a non-Centurion level.
    fn identify_trails_and_weights_for_member_level(&self,
                                                    member_depth: Depth,
                                                    c_depth: Depth,
                                                    (p_depth, previous_centurion): (Depth, &NWAreaLevel),
    ) -> NWAreaLevel {
        debug_assert!(c_depth < member_depth);
        debug_assert!(member_depth < p_depth);

        // Step from Centurion to member
        let m_step = NonZeroUsize::new(member_depth - c_depth).unwrap();
        // Step from member to the given Centurion
        let short_step = NonZeroUsize::new(p_depth - member_depth).unwrap();

        debug_assert_eq!(c_depth + m_step.clone().get() + short_step.clone().get(), p_depth);

        let mut m_level: NWAreaLevel = HashMap::default();
        for c_id in self.levels[c_depth].get_nodes().keys() {

            // We want to know the intersection of dependencies between Centurion and Member
            // Therefore, for each node in the Centurion, we first step down to the dependencies
            // in Member, and then onwards to the dependencies in previous Centurion reachable from
            // those original dependencies in Member.
            for (m_id, c_edge) in DepBoolFinder::new(*c_id, c_depth,
                                                     m_step,
                                                     self) {


                let m_weight = DepBoolFinder::new(m_id,
                                                  member_depth,
                                                  short_step,
                                                  self)
                    .iter()
                    .map(|(p_id, m_edge)| {
                        let edge = c_edge | m_edge;
                        let weight = previous_centurion.get(p_id).expect("Using an outdated arena?");
                        if edge {
                            weight << 1
                        } else {
                            *weight
                        }
                    })
                    .fold(0, |acc, weight|{ acc | weight });

                let current_weight = m_level.entry(m_id). or_insert(0);
                *current_weight |= m_weight;
            }
        }

        m_level
    }


    // perform a BFS with node_id as root, and down to 'depth' levels below (depth == step)
    // Grab those nodes' count, leftshift one time (i.e. multiply with 2) the individual count iff
    // at least one '1-edge' was traversed. Then OR them all together, and return the result.
    fn calculate_trail_weights_for_node(&self, node_id: &Id, node_depth: usize, step: NonZeroUsize,
                                        level: &NWAreaLevel) -> u128
    {
        // Find all nodes reachable from root, located in level with index (depth + step)
        let deps = DepBoolFinder::new(*node_id, node_depth, step, &self);
        // Then calculate the trails weights for root
        deps.iter()
            .map(|(id, one_edge)| (level.get(&id).unwrap(), one_edge))
            .map(|(trails, one_edge)| {
                // shift bits once left iff we traversed at least one 1-edge as part of the path
                if *one_edge { trails * 2 }
                else { *trails }
            })
            .fold(
                0,
                |acc, trails| { acc | trails }
            )
    }


    /// Counts trails present in a node, not only recording the presence. Does this only for the
    /// Centurions in active_area. Active_area.start is expected to be a Centurion, and the len of
    /// active area is expected to be a multiple of step.
    fn count_trails_and_weights_core(&self, sub_active_area: &Range<usize>, step: usize) -> PWCArenaLevel {

        assert_ne!(0, step, "Step cannot be 0!");
        let nz_step = NonZeroUsize::new(step).unwrap();
        let top = sub_active_area.start; // Inclusive
        let bottom = sub_active_area.end; // Exclusive

        // The part of the shard which is relevant to us, and which satisfies the module level invariants.
        let work_area = &self.levels[top..bottom];

        // Get index to "base case": Lowest level in the shard that will have weights associated with it
        let base_case_index = bottom - step;


        // We need to take into account that previous_centurion may be the sink node.
        if top == bottom {
            return PWCArenaLevel::new_from(
                self.levels[bottom].get_nodes()
                    .keys()
                    .map(|id| (*id, PWCount::fresh_trivial()))
                    .collect()
            )
        }


        // Init "base case": Go `step` down, and see if an 1-edge or more was traversed, set weight accordingly
        let base_case: HashMap<Id, PWCount, BuildHasherDefault<ahash::AHasher>> =
            work_area[base_case_index - top].get_nodes().keys()
                .map(|id| (id, DepBoolFinder::new(*id, base_case_index, nz_step,
                                                  self)))

                .map(|(id, deps)| {
                    // OR together the correct weights for each path
                    let trails = deps.iter()
                        //Did we pass at least one 1-edge on the way to id?
                        .map(|(_id, edge)| if *edge { PWCount::fresh_one() }
                        else { PWCount::fresh_trivial() }
                        )
                        .fold(PWCount::new(),
                              |acc, weight| { acc + weight });
                    (*id, trails)
                }).collect();

        let base_case = PWCArenaLevel::new_from(base_case);
        // Are we done?
        if bottom - top == step {
            return base_case;
        }

        // Since the underlying HashMap for a PWCount may have up to CAPACITY (expected to be 128)
        // instances of a BigUInt, since I have no idea if a BigUInt even has a limit to its size,
        // and since we may have above soft_lim shards when we do the counting, I've opted to not
        // fill an arena with every Centurion level, and instead only calculate the values for the
        // requested Centurion. Since we always need the level step levels below in order to calculate
        // values in current, I've opted to alternate between filling and draining two levels:

        // We need two vec's to work with, one is filled while the other is referenced. (Saves memory).
        // Called 'zero' and 'one', to make it easy to remember which is which w.r.t. index % 2.
        let mut zero = base_case;
        let mut one = PWCArenaLevel::new_from(HashMap::default());
        let mut even = true;

        debug_assert_eq!(0, self.levels[top..(base_case_index-step)].len() % step);
        // Step through the working area (bottom up), calculating the weights for the nodes
        // at `step` intervals.
        for (i, level) in self.levels[top..=(base_case_index-step)].iter().rev()
            .step_by(step).enumerate() {

            // Setting up the correct vec's for fill and reference. Remember to empty fill first!
            let (prev, fill) = match even {
                true => {
                    one.clear();
                    (&mut zero, &mut one)
                },
                false => {
                    zero.clear();
                    (&mut one, &mut zero)
                },
            };

            let node_depth = base_case_index - step*(i+1);
            for id in level.get_nodes().keys() {
                // Calculate weights
                let counts = self.calculate_trail_counts_for_node(id, node_depth,
                                                                  nz_step, prev);
                // insert into fill
                fill.insert(*id, counts);
            }
            even = !even;
        }

        // Return the HashMap which was last filled.
        match even {
            true => zero,
            false => one,
        }
    }

    fn count_trails_and_weights_for_member_level(&self,
                                                 member_depth: Depth,
                                                 c_depth: Depth,
                                                 (p_depth, previous_centurion): (Depth, &PWCArenaLevel),
    ) -> PWCArenaLevel {
        debug_assert!(c_depth < member_depth);
        debug_assert!(member_depth < p_depth);

        // Step from Centurion to member
        let m_step = NonZeroUsize::new(member_depth - c_depth).unwrap();
        // Step from member to the given Centurion
        let short_step = NonZeroUsize::new(p_depth - member_depth).unwrap();

        debug_assert_eq!(c_depth + m_step.clone().get() + short_step.clone().get(), p_depth);

        let mut m_level: PWCArenaLevel = PWCArenaLevel::new_from(HashMap::default());
        for c_id in self.levels[c_depth].get_nodes().keys() {
            for (m_id, c_edge) in DepBoolFinder::new(*c_id, c_depth,
                                                     m_step,
                                                     self) {

                let mut m_count = PWCount::new();
                for (p_id, m_edge) in DepBoolFinder::new(m_id,
                                                         member_depth,
                                                         short_step,
                                                         self)
                    .iter() {
                    let edge = c_edge | m_edge;
                    let mut counts = previous_centurion.get(p_id)
                        .expect("Using an outdated arena?")
                        .clone();
                    if edge {
                        counts.increment_indices();
                    }
                    m_count += counts;
                }
                // Update existing count
                let existing_count = m_level.entry(m_id).or_insert(PWCount::new());
                *existing_count += m_count;
            }
        }

        m_level
    }


    /// Id the weights v and count the paths with weight v passing through the node with node_id.
    fn calculate_trail_counts_for_node(&self, node_id: &Id, node_depth: usize, step: NonZeroUsize,
                                       prev_level: &mut PWCArenaLevel) -> PWCount{
        // Find all nodes reachable from root, located in level with index (depth + step)
        let deps = DepBoolFinder::new(*node_id, node_depth, step, &self);
        // Then calculate the trails weights for root
        let mut counts = PWCount::new();

        for (id, one_edge) in deps.iter() {
            let mut trails = prev_level.get(&id).unwrap().clone();
            if *one_edge { trails.increment_indices(); }
            counts += trails;
        }

        counts
    }

    /// Deletes *all* the given nodes at the given level depth, only updating parents and
    /// executing the reduction algorithm *after all* the given nodes have been deleted.
    ///
    /// NOTE: We suspect that the reduction algorithm may fail in some corner cases. More precisely,
    /// the 'merge op' may fail to merge all isomorphic nodes in the graph under certain conditions.
    /// This should in general not be a problem.
    ///
    /// **Will not delete the root node.** (Will return without doing anything).
    ///
    ///
    /// Panics: (May be changed to Err in the future).
    /// - If the index (depth) is out of bounds.
    pub fn delete_all_marked_nodes_from_level(&mut self, delete: HashSet<&Id>, depth: usize) {
        if depth == 0 {
            return;
        }
        if delete.is_empty() {
            return;
        }

        // Remove nodes
        {
            let children = self.levels.get_mut(depth)
                .expect("Level does not exist")
                .get_mut_nodes();
            for id in delete.iter() {
                // Remove the node
                children.remove(id);
            }
        }


        // Update parents. I.e. remove dangling edges.
        let parents = self.levels.get_mut(depth -1).unwrap();
        for (_id, node) in parents.iter_mut_nodes() {
            if let Some(e0) = node.get_e0() {
                if delete.contains(&e0) {
                    node.disconnect_e0();
                }
            }
            if let Some(e1) = node.get_e1() {
                if delete.contains(&e1) {
                    node.disconnect_e1();
                }
            }
        }

        // Reduce: FIXME, took a closer look at these methods, and realized that they may sometimes
        // short circuit too early. Particularly the last one, merging equal nodes, is under
        // suspicion. Given that dead end and orphan removal have been done, then it may
        // be equal nodes on non adjacent levels.

        //Reduce.
        self.remove_all_dead_ends_start(depth -1);
        self.remove_orphans_start(depth + 1);
        self.merge_equals_node_start(depth-1); // TODO verify
    }




    /// Convert a `RangeBounds` into a `Range`, taking into account this `Shards`'s length and sink
    /// level. Also checks that the range is larger than or equal to `step`.
    fn process_range<R>(&self, range: R, step: usize) -> Range<usize>
        where
            R: RangeBounds<usize>,
    {
        use Bound::{Included, Excluded, Unbounded};

        let len = self.get_levels_size();
        let top = match range.start_bound() {
            Included(&n) => n,
            Excluded(&n) => n + 1,
            Unbounded => 0,
        };

        let bottom = match range.end_bound() {
            Included(&n) => n + 1,
            Excluded(&n) => n,
            Unbounded => len - 1, // Taking sink into account => '-1'.
        };

        // Invariant checks
        if top > bottom  || (bottom - top) < step {
            panic!("Top index should be less than bottom AND (bottom - top) >= step\n\
                 Top: {}. Bottom: {}. Step: {}", top, bottom, step)
        }
        if bottom >= len {
            panic!("Bottom index should be <= self.len(). Bottom: {}. len: {}", bottom, len);
        }

        Range{ start: top, end: bottom }
    }
}

// =============================================================================================
// =============================== Delete nodes Complexity based ===============================
// =============================================================================================
impl Bdd {
    /// Deletes the given nodes at the given level (depth), until:
    /// 1) All the given nodes have all been deleted OR:
    /// 2) The complexity of self has been reduced below `complexity_target`.
    ///
    /// Relevant parents will be updated, and the reduction algorithm will be run after *each* node
    /// deletion.
    ///
    /// **Will not delete the root node.** (Will return without doing anything).
    ///
    /// **IMPORTANT**: We give no guarantees for what order the nodes will be deleted in. This
    /// implies that a node with a lower LSB may be deleted *before* some other node with a higher
    /// LSB. This may matter whenever this method returns early (aka before 1) is met).
    ///
    /// NOTE: We suspect that the reduction algorithm may fail in some corner cases. More precisely,
    /// the 'merge op' may fail to merge all isomorphic nodes in the graph under certain conditions.
    /// This should in general not be a problem.
    ///
    /// Returns a map with the Id's of any deleted nodes, along with their weight.
    ///
    /// Panics:
    /// - If the index (`depth`) is out of bounds.
    fn delete_nodes_from_level_until(&mut self,
                                     complexity_target: usize,
                                     delete: Vec<Id>,
                                     // delete: HashMap<Id, u128, BuildHasherDefault<AHasher>>,
                                     depth: usize,
                                     step: usize,
                                     loop_logger: &mut PruneLoopRecordBuilder,
    )
    {
        // For logging purposes:
        // let mut deleted: HashMap<Id, u128, BuildHasherDefault<AHasher>> = Default::default();
        let nodes_at_level = self.levels.get(depth).unwrap().get_nodes_len() as f64;
        let nr_marked = delete.len() as f64;

        let mut dd_rec = loop_logger.new_depth_deletion_builder(depth,
                                                                nodes_at_level as usize,
                                                                nr_marked as usize,
                                                                self.get_size(),
                                                                complexity_target);

        if depth == 0 {
            // Register the complete record with parent logger
            loop_logger.register_depth_deletion_rec(
                dd_rec.finalize(self.get_size()));
            return
        }
        if delete.is_empty() {
            // Register the complete record with parent logger
            loop_logger.register_depth_deletion_rec(
                dd_rec.finalize(self.get_size()));
            return;
        }
        if self.get_size() < complexity_target {
            // Register the complete record with parent logger
            loop_logger.register_depth_deletion_rec(
                dd_rec.finalize(self.get_size()));
            return;
        }

        // Mapping children to parents
        let mut child_parent_map = self.children_parent_map(depth - 1);


        // Performing initial batch calculations
        let mut since_last = 0; // Deleted nodes since last reduce
        let mut complexity_batch_start = self.get_size();
        // Estimated nr of nodes removed for the DAG when a node is deleted.
        // (Including the node that was deleted)
        let mut deletion_rate = step as f64;
        // Run next reduce when we've deleted "reduce at " nr of nodes
        let mut batch_size = Self::calculate_batch_size(complexity_target,
                                                        complexity_batch_start,
                                                        deletion_rate,
                                                        nr_marked);

        let mut batch_rec = dd_rec.new_batch_builder(batch_size,
                                                     complexity_batch_start,
                                                     deletion_rate);

        let mut marked_remaining = nr_marked;
        let mut missed_removed = 0;


        // Node removal loop
        for child_id in delete.iter() {

            // Remove the node
            if
            self.levels.get_mut(depth).unwrap()
                .get_mut_nodes()
                .remove(child_id)
                // Nodes marked on other levels may have been removed as part of a reduce op
                .is_none()
            {
                marked_remaining -= 1_f64;
                missed_removed += 1;
                continue
            }
            marked_remaining -= 1_f64;
            since_last += 1;


            // Update parents. I.e. remove dangling edges.
            let my_parents = child_parent_map.get_mut(child_id)
                .expect("Seemed to have encountered an unexpected orphan");
            for parent_id in my_parents.iter() {
                // Quickfix to make the compiler happy. Would like a way not to do all these repeated calls...
                let parent = self.levels.get_mut(depth - 1).unwrap()
                    .get_mut_nodes()
                    .get_mut(parent_id);

                // Parent found, update edge(s).
                if parent.is_some() {
                    let parent = parent.unwrap();
                    if let Some(e0) = parent.get_e0() {
                        if &e0 == child_id {
                            parent.disconnect_e0();
                        }
                    }
                    if let Some(e1) = parent.get_e1() {
                        if &e1 == child_id {
                            parent.disconnect_e1();
                        }
                    }
                } // else:
                // Parent may have merged with another parent, invalidation this 'parent' Id:
                // continue;
            }

            // About Reduce: FIXME, took a closer look at these methods, and realized that they may sometimes
            // short circuit too early. Particularly the last one, merging equal nodes, is under
            // suspicion. Given that dead end and orphan removal have been done, we may have
            // equal nodes (aka merge-able nodes) on non adjacent levels.

            // Batch done, do a reduce op, then logg
            if since_last == batch_size {
                // Reduce:
                self.remove_all_dead_ends_start(depth - 1);
                self.remove_orphans_start(depth + 1);
                self.merge_equals_node_start(depth - 1); // TODO verify
                // Update vars
                deletion_rate = (complexity_batch_start - self.get_size()) as f64 / batch_size as f64;
                complexity_batch_start = self.get_size();

                dd_rec.register_batch(
                    batch_rec.finalize(deletion_rate, since_last, missed_removed));
                missed_removed = 0;
                // Are we done?
                if self.get_size() < complexity_target {
                    // Register the complete record with parent logger
                    loop_logger.register_depth_deletion_rec(
                        dd_rec.finalize(self.get_size()));
                    return
                    // return deleted;
                }

                // else set up next batch:
                batch_size = Self::calculate_batch_size(complexity_target,
                                                        complexity_batch_start,
                                                        deletion_rate,
                                                        marked_remaining);
                since_last = 0;
                // New logg entry
                batch_rec = dd_rec.new_batch_builder(batch_size,
                                                     complexity_batch_start,
                                                     deletion_rate);

            }
        }
        // Final reduce, since we ran out if nodes to delete
        self.remove_all_dead_ends_start(depth - 1);
        self.remove_orphans_start(depth + 1);
        self.merge_equals_node_start(depth - 1); // TODO verify

        deletion_rate = (complexity_batch_start - self.get_size()) as f64 / batch_size as f64;
        dd_rec.register_batch(
            batch_rec.finalize(deletion_rate, since_last, missed_removed));

        // Register the complete record with parent logger
        loop_logger.register_depth_deletion_rec(
            dd_rec.finalize(self.get_size()));

        // deleted
    }

    /// Crate a mapping from each child to its parents, i.e. we make indirect edges from child to
    /// parents.
    fn children_parent_map(&mut self, parents_depth: usize) -> HashMap<Id, HashSet<Id>, BuildHasherDefault<AHasher>> {
        let mut child_parent_map: HashMap<Id, HashSet<Id>, BuildHasherDefault<AHasher>> = Default::default();

         self.levels
             // Get nodes on parent level
             .get_mut(parents_depth ).unwrap()
             .get_mut_nodes()
             // create mapping
             .iter()
             .for_each(|(parent_id, node)| {

                 if let Some(e0) = node.get_e0() {
                     let parents = child_parent_map.entry(e0)
                         .or_insert(Default::default());
                     parents.insert(parent_id.clone());
                 }

                 if let Some(e1) = node.get_e1() {
                     let parents = child_parent_map.entry(e1)
                         .or_insert(Default::default());
                     parents.insert(parent_id.clone());
                 }
             });
        child_parent_map
    }

    /// Calling reduce after every node deleting in the finer grained pruning process is
    /// costly, with approximately 10~100 deletions per second.
    ///
    /// We therefore wish to estimate when to run reduce in a dynamical fashion.
    ///
    /// `deletion_rate` is expected number of nodes to be deleted per node deleting. I.e. the
    /// deleted node itself plus any nodes that will/would be removed as part of reduce. This is
    /// an estimate.
    ///
    /// Returns how many nodes to delete before running reduce, i.e. the batch size.
    fn calculate_batch_size(complexity_target: usize,
                            current_complexity: usize,
                            deletion_rate: f64,
                            remaining_marked: f64, )
                            -> usize
    {
        // Half of the difference between current complexity and complexity target, adjusted
        // by how many nodes we expect to be part of the 'avalanche' effect (including the
        // marked node itself).
        let batch_size_complexity_based =
            ((current_complexity - complexity_target) as f64 / (deletion_rate * 2_f64)) as usize;

        // Half of the remaining nodes marked for deletion
        let batch_size_marked_based = (remaining_marked / 2_f64) as usize;

        // Return the smallest of complexity based and marked based, but no less than 1.
        1_usize.max(batch_size_complexity_based.min(batch_size_marked_based))
    }
}







// =============================================================================================
// ======================================== Mod Test ===========================================
// =============================================================================================
#[cfg(test)]
mod test;



