// #[cfg(feature = "unstable")]
// use std::collections::TryReserveError;

pub use arenas::{WDArena, WDLevel};
pub use distribution::{NcWDistribution, Node2NodeDistribution, NWDistribution};
pub use distribution::count::WDCount;
pub use distribution::count_v2::WDCountV2;
pub use distribution::dist_factories::*;
pub use distribution::end_node::EndNodeDist;
pub use distribution::presence::WDPresence;
use std::num::NonZeroUsize;
use std::ops::Range;

use crate::soc::bdd::Bdd as Shard;
use crate::soc::bdd::differential::{PPFactory, StyledProgressBar};
use crate::soc::bdd::differential::dependency_finder::DepBoolFinder;
use crate::soc::Id as NodeId;

use super::Depth;

/// Number of Paths associated with a weight. Present in a weight distribution in a Node.
pub type PathCount = u64;

mod arenas;
mod distribution;

// Consider todo: Make a mod with fully parallelized versions of relevant fn's. Perhaps
// feature gated to save compile time?

//FIXME handle edge case "in last cohort". (Use base case's values). See code-comments in fill_base_case(...)

impl Shard {

    /// Calculates the weight distributions for the level at 'depth'. Will work its way from the
    /// end of the active area, up to and including the level at 'depth', before returning the level
    /// for 'depth'.
    ///
    /// Will Panic at any index out of bounds, and on an edge-case (see in-code notes on base-case).
    pub fn weight_distributions_for_level<W, F>(&self,
                                                depth: Depth,
                                                active_area: &Range<usize>,
                                                step: NonZeroUsize,
                                                factory: &F)
                                             -> WDLevel<W>
        where W: NWDistribution, F: DistFactory<W> {
        assert!(depth < active_area.end, "Cannot find weights below of active area");
        assert!(depth >= active_area.start, "Cannot find weights above of active area");


        // This is a three part process, depending on 'depth':
        // 1) Filling base case (the lowest Centurion which we can find distributions for).
        //      First possibility of return: depth == base_case_index.
        // 2) Work our way up active area (= downwards in indexes).
        //      Second chance of return: If depth is a centurion, then it will be returned here.
        // 3) Depth is a member, and we've filled the Centurion closest below it. Now we work to
        //    fill depth. The process will terminate here, as this is the latest point where depth
        //    can be filled.

        // NOTE: There is an edge case which isn't covered here: When Depth is between base case
        // and sink. (base_case < depth < sink). As this is highly unlikely to happen when pruning
        // any SoC build from an SPN cipher, we've opted to (for now) panic on if this were to happen.
        // This is also noted in the fn fill_base_case).

        // ===================================================================================
        // Part 1)
        // Filling base-case
        let (base_case_index, base_case) = self.fill_base_case(depth,
                                                               &active_area,
                                                               step,
                                                               factory);
        // Are we done?
        if base_case_index == depth {
            return base_case;
        }

        // =============================================================================
        // Part 2)
        // Filling intermediate Centurions

        // We use two vec's for this work, one is filled while the other is the previous Centurion.
        // (Reuses memory).
        // I call them 'zero' and 'one', to make it easy to remember which is which w.r.t. index % 2.
        let mut zero = base_case;
        let mut one: WDLevel<W> = WDLevel::new(None);
        // Keep track of the roles of zero and one.
        let mut even = true;

        // If depth is a Centurion, we fill it and return it. Otherwise, we stop at previous
        // Centurion, and continue to part 3.

        let mut p_depth = base_case_index;

        loop {
            let current_depth = p_depth - step.get();
            // Done if we've passed target depth:
            // p_depth is then either the depth of the previous Centurion, of depth is itself a
            // Centurion, and was the last one filled
            if depth > current_depth {
                break;
            }

            // Setting up the correct vec's for fill and reference. Remember to empty fill first!
            let (prev, fill) = match even {
                true => {
                    one.clear();
                    one.set_depth(current_depth);
                    (&mut zero, &mut one)
                },
                false => {
                    zero.clear();
                    zero.set_depth(current_depth);
                    (&mut one, &mut zero)
                },
            };

            // Fill 'fill': Centurion for current depth
            for node_id in self.levels[current_depth].get_nodes().keys() {
                // Calculate weights
                let distribution = self.calculate_distribution_for_node(node_id, current_depth,
                                                                        step, prev);
                // insert into fill,
                fill.insert(*node_id, distribution);
            }


            // Is depth itself a Centurion? (This one also handles if depth == zero).
            if current_depth == depth {
                let ret = match even {
                    true => one,
                    false => zero,
                };
                return ret
            }

            // Update p_depth and switch the roles of zero and one.
            p_depth = current_depth;
            even = !even;
        }

        // ====================================================================================
        // Part 3)
        // We now know that depth is a member, and that it is between current depth and
        // current depth + step.
        let previous_centurion = match even {
            true => zero,
            false => one,
        };
        self.fill_distributions_for_member(depth,
                                           p_depth - step.get(),
                                           (p_depth, &previous_centurion))
    }

    /// Calculates and keeps all the WDLevels for all the Centurions between active_area.end and
    /// 'depth' (inclusive).
    /// This function will keep the calculated weight distributions for all these levels in an
    /// WDArena. 'Depth', along with all the Centurions below 'depth', are guaranteed to be in
    /// the Arena. The Centurion of the cohort 'depth' is a member of, will be in the Arena **iff**
    /// 'depth' is the Centurion.
    ///
    /// WARNING! This may be VERY memory intensive, depending on what weight distribution 'W' is
    /// utilized. WDPresence is designed with this in mind, and is the least memory intensive at the
    /// time of this writing.
    ///
    /// Will Panic at any index out of bounds, and on an edge-case (see notes on base-case).
    pub fn weight_distributions_arena_for_level<W, F>(&self, depth: Depth,
                                                      active_area: &Range<usize>,
                                                      step: NonZeroUsize,
                                                      factory: &F,
    )
                                                   -> WDArena<W>
    where W: NWDistribution, F: DistFactory<W> {

        // This is a three part process, depending on 'depth':
        // 1) Filling base case (the lowest Centurion which we can find distributions for).
        //      First possibility of return: depth == base_case_index.
        // 2) Work our way up active area (= downwards in indices).
        //      Second chance of return: If depth is a centurion, then the we will be return here.
        // 3) Depth is a member, and we've filled the Centurion closest below it. Now we work to
        //    fill depth.
        //    The process will terminate here, as this is the latest point where depth
        //    can be filled.

        // NOTE: There is an edge case which isn't covered here: When Depth is between base case
        // and sink. (base_case < depth < sink). As this is highly unlikely to happen when pruning
        // any SoC build from an SPN cipher, we've opted to (for now) panic on if this were to happen.
        // This is also noted in the fn fill_base_case).

        // ===================================================================================
        // Part 1)
        // Filling base-case
        let (base_case_index, base_case) = self.fill_base_case(depth,
                                                               &active_area,
                                                               step,
                                                               factory);

        let mut arena = WDArena::new();
        arena.insert_level(base_case_index, base_case);

        // Are we done?
        if base_case_index == depth {
            return arena;
        }

        // =============================================================================
        // Part 2)
        // Filling intermediate Centurions
        let mut p_depth = base_case_index;
        loop {
            let current_depth = p_depth - step.get();
            // Done if we've passed target depth:
            // p_depth is then either the depth of the previous Centurion, of depth is itself a
            // Centurion, and was the last one filled
            if depth > current_depth {
                break;
            }
            // Insert next centurion
            arena.insert_level(current_depth,
                               self.fill_distributions_for_centurion(step,
                                                                     current_depth,
                                                                     (p_depth, arena.get(&p_depth).unwrap())),
            );

            // Is depth itself a Centurion? (This one also handles if depth == zero).
            if current_depth == depth {
                return arena;
            }

            // Update p_depth
            p_depth = current_depth;
        }

        // Part 3:
        // depth is a member depth, p_depth is indeed the Centurion of the Cohort directly below
        // depth's own Cohort,
        arena.insert_level(depth, self.fill_distributions_for_member(depth,
                                                              p_depth - step.get(),
                                                              (p_depth, arena.get(&p_depth)
                                                                  .unwrap())));
        // We are done
        arena
    }




    /// Finds and fills the 'base case' level: The first Centurion for which we can fill any weight
    /// distributions in its nodes.
    /// Returns the depth of the base case, and the base case level itself.
    ///
    /// Panics! There is an edge case where depth is between the base case and active_area.end,
    /// which currently will result in a panic. This edge case is expected to just that, an edge case,
    /// and thus isn't handled properly yet.
    fn fill_base_case<W, F>(&self,
                            depth: Depth, // depth is only relevant to check an edge case we don't support!
                            active_area: &Range<usize>,
                            step: NonZeroUsize,
                            factory: &F,
    ) -> (usize, WDLevel<W>)
        where W: NWDistribution,
              F: DistFactory<W>
    {
        // Get index to "base case": Lowest level in the shard that will have weights associated with it
        let base_case_index = active_area.end - step.get();

        assert!(depth <= base_case_index,
                "Congratulations! You've hit an edge-case which we currently doesn't support:\
    \nYour depth is between the base-case and end of active area.\
    \nYour depth: {}. Base case index: {}. Active area start: {}, end {}. ",
                        depth,  base_case_index, active_area.start, active_area.end);

        // Init "base case": Go `step` down, and see if an 1-edge or more was traversed,
        // set distribution accordingly
        let mut base_case: WDLevel<W> =
            self.levels[base_case_index].get_nodes().keys()
                .map(|id| {
                    (*id, bc_distribution(DepBoolFinder::new(*id,
                                                             base_case_index,
                                                             step,
                                                             self), factory))
                })
                .collect();
        base_case.set_depth(base_case_index);


        #[inline(always)]
        /// Given a set of node dependencies, will create a base-case distribution for that root.
        // Made its own fn for readability in the above mapping. And also as an experiment, because
        // I just learned that this is possible, but I don't yet know when or why one'd make one of
        // these "inner" fn's (or whatever they are called)...
        fn bc_distribution<W, F>(deps: DepBoolFinder, factory: &F) -> W
            where W: NWDistribution, F: DistFactory<W>
        {
            let distribution = deps.iter()
                //Did we pass at least one 1-edge on the way to id?
                .map(|(id, edge)|
                    if *edge {
                        let mut t = factory.new_trivial(id);
                        t.increment_distribution();
                        t
                    }
                    else {
                        factory.new_trivial(id)
                    }
                )
                .fold(W::new_zeroed(),
                      |acc, weight| { acc + weight });
            distribution
        }

        (base_case_index, base_case)
    }

    /// Returns a WDLevel for a *Centurion level*.
    /// This is different than filling a WDLevel for a *member level*, which is handled in its own fn.
    #[inline]
    fn fill_distributions_for_centurion<W>(&self,
                                           step: NonZeroUsize,
                                           centurion_depth: Depth,
                                           (p_depth, previous_centurion): (Depth, &WDLevel<W>)
    ) -> WDLevel<W>
    where W: NWDistribution
    {
        debug_assert!(centurion_depth < p_depth);
        debug_assert_eq!(p_depth - centurion_depth, step.get());

        let mut c_level: WDLevel<W> = WDLevel::new(Some(centurion_depth));
        for c_id in self.levels[centurion_depth].get_nodes().keys() {

            let distribution = self.calculate_distribution_for_node(c_id,
                                                                    centurion_depth,
                                                                    step,
                                                                    previous_centurion);

            // Update existing count
            c_level.insert(*c_id, distribution);
        }
        c_level
    }

    /// Returns a WDLevel for a *non-Centurion* level. Note that this has to be handled slightly
    /// different from a centurion level.
    /// There exists an equivalent fn which handles Centurion levels.
    #[inline]
    fn fill_distributions_for_member<W>(&self,
                                        member_depth: Depth,
                                        c_depth: Depth,
                                        (p_depth, previous_centurion): (Depth, &WDLevel<W>)
    ) -> WDLevel<W>
        where W: NWDistribution {
        debug_assert!(c_depth < member_depth, "c_depth: {}, member_depth: {}", c_depth, member_depth);
        debug_assert!(member_depth < p_depth, "member_depth: {}, p_depth: {}", c_depth, p_depth);

        // Step from (member's) Centurion to member itself
        let cm_step = NonZeroUsize::new(member_depth - c_depth).unwrap();
        // Step from member to the previous Centurion
        let mpc_step = NonZeroUsize::new(p_depth - member_depth).unwrap();

        debug_assert_eq!(c_depth + cm_step.clone().get() + mpc_step.clone().get(), p_depth);

        let mut m_level: WDLevel<W> = WDLevel::new(Some(member_depth));
        for c_id in self.levels[c_depth].get_nodes().keys() {
            // We want to know the intersection of dependencies between Centurion and Member
            // Therefore, for each node in the Centurion, we first step down to the dependencies
            // in Member, and then onwards to the dependencies in previous Centurion reachable from
            // those original dependencies in Member.
            for (m_id, c_edge) in DepBoolFinder::new(*c_id, c_depth,
                                                     cm_step,
                                                     self) {

                let mut m_distribution = W::new_zeroed();
                for (p_id, m_edge) in DepBoolFinder::new(m_id,
                                                         member_depth,
                                                         mpc_step,
                                                         self)
                    .iter() {
                    let edge = c_edge | m_edge;
                    let mut p_distribution = previous_centurion.get(p_id).expect("Using an outdated arena?")
                        .clone();
                    if edge {
                        p_distribution.increment_distribution();
                    }
                    m_distribution += p_distribution;
                }
                // Update existing count, it may have been created by another node in Centurion
                let existing_count = m_level.entry(m_id).or_insert(W::new_zeroed());
                *existing_count += m_distribution;
            }
        }

        m_level
    }

    /// Find the weight distribution for 'node_id'.
    ///
    /// Panics if it is not possible to get the distribution of one of the nodes this node is
    /// "dependent" on. (See DependencyFinder for def of "dependent on").
    #[inline]
    fn calculate_distribution_for_node<W>(&self,
                                          node_id: &NodeId,
                                          node_depth: usize,
                                          step: NonZeroUsize,
                                          previous_centurion: &WDLevel<W>)
                                          -> W
        where W: NWDistribution {
        // Find all nodes reachable from root, where the non-root nodes are located in the centurion
        // with index (node_depth + step) (Centurion in Cohort immediately below).
        let deps = DepBoolFinder::new(*node_id, node_depth, step, &self);

        // Then calculate the distribution for node_id
        let mut distribution = W::new_zeroed();

        for (id, one_edge) in deps.into_iter() {
            let mut p_distribution = previous_centurion.get(&id)
                .expect("Using an outdated arena, or perhaps the wrong depth?")
                .clone();
            if one_edge { p_distribution.increment_distribution(); }
            distribution += p_distribution;
        }
        distribution
    }


    /// Same as weight_distributions_for_level, but building from top level to bottom level instead.
    /// 'Top' is active_area.start.
    /// 'bottom', the level we return, is 'Depth'
    /// TODO rename!
    /// Some invariants:
    /// - levels are organized in Cohorts
    /// - active_area.start is assumed to be a Centurion
    /// - 'depth' is within active_area
    /// - active_area.start will be the starting level we construct from
    pub fn weight_distributions_for_level_top_bottom<W, F, P>(&self,
                                                              depth: Depth,
                                                              active_area: &Range<usize>,
                                                              step: NonZeroUsize,
                                                              factory: &F,
                                                              pb_factory: &P,
    )
                                                              -> WDLevel<W>
        where W: NWDistribution, F: DistFactory<W>, P: PPFactory,
    {
        assert!(depth < active_area.end, "Cannot find weights below of active area");
        assert!(depth >= active_area.start, "Cannot find weights above of active area");

        let pb = pb_factory.new_progress_bar(
            ((depth - active_area.start)/ step.get()) as u64 + 1 );
        pb.set_message("Filling base case");

        // Building from top to bottom is a three part process:
        // 1) Init active_area.start as "base case"
        //      First possibility of return: 'depth' == active_area.start.
        // 2) Work our way down the active area (= increasing in indexes), until we hit the closest
        //    Centurion above (or equal to) 'depth'.
        //      Second chance of return: If 'depth' is a centurion, then it will be returned here.
        // 3) 'Depth' is a member, and we've filled the Centurion closest above it. Now we work to
        //    fill 'depth'. The process will terminate here, as this is the latest point where 'depth'
        //    can be filled.

        // ===================================================================================
        // Part 1): Filling base case
        let top: WDLevel<W> = self.levels[active_area.start]
            .get_nodes()
            .keys()
            .map(|id| (id.clone(), factory.new_trivial(id)))
            .collect();

        // Are we done?
        if active_area.start == depth {
            pb.finish_and_clear();
            return top;
        }
        pb.inc(1);

        // ===================================================================================
        // Part 2
        // Filling intermediate Centurions
        pb.set_message("Filling Centurions");

        // We use two vec's for this work, one is filled while the other is the previous Centurion.
        // (Originally done to reuse memory. Now we make a new WDLevel with capacity each turn instead,
        // to free up memory once we've passed the widest levels).
        // I call them 'zero' and 'one', to make it easy to remember which is which w.r.t.
        // iterations % 2.
        let mut zero = top;
        let mut one: WDLevel<W> = WDLevel::new(None);
        // Keep track of the roles of zero and one.
        let mut even = true; // First "fill" will be 'one'

        // If depth is a Centurion, we fill it and return it. Otherwise, we stop at previous
        // Centurion, and continue to part 3.
        let mut p_depth = active_area.start;

        let centurion_above =
            loop {
                let current_depth = p_depth + step.get();
                // Break if we've passed target depth:
                // If depth is itself a Centurion then we will return instead of breaking
                // (done after 'fill' is filled).
                if depth < current_depth {
                    break match even {
                        false => one,
                        true => zero,
                    };
                }

                // Setting up the correct vec's for fill and reference. Remember to empty fill first!
                let (prev, mut fill) = match even {
                    true => {
                        one = WDLevel::with_capacity(self.level(p_depth).unwrap().get_nodes_len());
                        (&mut zero, &mut one)
                    },
                    false => {
                        zero = WDLevel::with_capacity(self.level(p_depth).unwrap().get_nodes_len());
                        (&mut one, &mut zero)
                    },
                };


                // Fill 'fill': Centurion for current depth
                for (node_id, node_dist) in prev.iter() {
                    self.update_distribution_for_nodes_below(node_id, node_dist, p_depth, step, &mut fill);
                }
                pb.inc(1);

                // We're done if 'depth' a Centurion
                if current_depth == depth {
                    let mut ret = match even {
                        true => one,
                        false => zero,
                    };
                    ret.set_depth(current_depth);
                    pb.finish_and_clear();
                    return ret;
                }

                // Update p_depth and switch the roles of zero and one.
                p_depth = current_depth;
                even = !even;
            };


        // ====================================================================================
        // Part 3)
        // We now know that depth is a member, and that it is between current depth and
        // current depth + step.

        pb.set_message("Filling member");
        // "revert" p_depth to centurion_above's depth
        p_depth += step.get();
        // Instantiate and fill the member
        let mut member: WDLevel<W> = WDLevel::new(None);
        for (node_id, node_dist) in centurion_above.iter() {
            self.update_distribution_for_nodes_below(node_id,
                                                     node_dist,
                                                     p_depth,
                                                     step,
                                                     &mut member);
        }
        // We're done!
        pb.finish_and_clear();
        member
    }



    /// For all nodes in 'level_to_update':
    /// Create or update the distribution of all nodes connected to 'start_node'.
    ///
    /// We cannot rely on the depth of 'levels_to_update' being set, and we therefore need to be
    /// informed how many levels below 'start_node' that 'level_to_update' lies.
    #[inline]
    fn update_distribution_for_nodes_below<W>(&self,
                                              start_node: &NodeId,
                                              start_node_dist: &W,
                                              start_node_depth: Depth,
                                              depth_difference: NonZeroUsize,
                                              level_to_update: &mut WDLevel<W>,
    )
        where W: NWDistribution
    {
        // Find all nodes reachable from 'start-node', at the level 'step' below
        let deps = DepBoolFinder::new(*start_node, start_node_depth, depth_difference, &self);

        // Update/create dists for each child node
        for (child_id, child_edge) in deps.into_iter() {
            let child_dist = level_to_update.entry(child_id)
                // This node is never meant to be tracked: We can safely bypass the Factory and create
                // a new_zeroed straight from W
                .or_insert(W::new_zeroed());
            let mut start_dist = start_node_dist.clone();
            // Increment start_node_dist iff we've traversed at least one one-edge
            if child_edge {
                start_dist.increment_distribution();
            }
            *child_dist += start_dist;
        }

    }
}
//
// #[cfg(feature = "unstable")]
// impl Shard {
//
//     /// Same as weight_distributions_for_level, but building from top level to bottom level instead
//     /// Some invariants:
//     /// - levels are organized in Cohorts
//     /// - active_area.start is assumed to be a Centurion
//     /// - 'depth' is within active_area
//     pub fn weight_distributions_for_level_top_bottom_unstable<W, F>(&self,
//                                                                     depth: Depth,
//                                                                     active_area: &Range<usize>,
//                                                                     step: NonZeroUsize,
//                                                                     factory: &F)
//                                                                     -> Result<WDLevel<W>, TryReserveError>
//
//         where W: NWDistribution, F: DistFactory<W>
//     {
//         assert!(depth < active_area.end, "Cannot find weights below of active area");
//         assert!(depth >= active_area.start, "Cannot find weights above of active area");
//
//         // Building from top to bottom is a three part process:
//         // 1) Init active_area.start as "base case"
//         //      First possibility of return: 'depth' == active_area.start.
//         // 2) Work our way down the active area (= increasing in indexes), until we hit the closest
//         //    Centurion above (or equal to) 'depth'.
//         //      Second chance of return: If 'depth' is a centurion, then it will be returned here.
//         // 3) 'Depth' is a member, and we've filled the Centurion closest above it. Now we work to
//         //    fill 'depth'. The process will terminate here, as this is the latest point where 'depth'
//         //    can be filled.
//
//         // ===================================================================================
//         // Part 1): Filling base case
//         let top: WDLevel<W> = self.levels[active_area.start]
//             .get_nodes()
//             .keys()
//             .map(|id| (id.clone(), factory.new_trivial(id)))
//             .collect();
//
//         // Are we done?
//         if active_area.start == depth {
//             return Ok(top);
//         }
//
//         // ===================================================================================
//         // Part 2
//         // Filling intermediate Centurions
//
//         // If depth is a Centurion, we fill it and return it. Otherwise, we stop at previous
//         // Centurion, and continue to part 3.
//         let mut p_depth = active_area.start;
//
//
//         let mut prev = top;
//
//         loop {
//             // Current depth = depth of level we're about to "fill".
//             let current_depth = p_depth + step.get();
//             // Break if we've passed target depth:
//             // If depth is itself a Centurion then we will return instead of breaking
//             // (done after 'fill' is filled).
//             if depth < current_depth {
//                 break;
//             }
//
//             // Try allocate room for next level
//             let mut fill = WDLevel::new(None);
//             fill.try_reserve(self.level(current_depth).unwrap().get_nodes_len())?;
//
//             // Fill 'fill': Centurion for current depth
//             for (node_id, node_dist) in prev.iter() {
//                 self.update_distribution_for_nodes_below(node_id, node_dist, p_depth, step, &mut fill);
//             }
//
//             // We're done if 'depth' a Centurion
//             if current_depth == depth {
//                 prev.set_depth(current_depth);
//                 return Ok(prev);
//             }
//
//             // Update p_depth and 'prev'
//             p_depth = current_depth;
//             prev = fill;
//         };
//
//         let centurion_above = prev;
//
//         // ====================================================================================
//         // Part 3)
//         // We now know that depth is a member, and that it is between current depth and
//         // current depth + step.
//
//         // "revert" p_depth to centurion_above's depth
//         p_depth += step.get();
//         // Instantiate and fill the member
//         let mut member: WDLevel<W> = WDLevel::new(None);
//         for (node_id, node_dist) in centurion_above.iter() {
//             self.update_distribution_for_nodes_below(node_id,
//                                                      node_dist,
//                                                      p_depth,
//                                                      step,
//                                                      &mut member);
//         }
//         // We're done!
//         Ok(member)
//     }
// }