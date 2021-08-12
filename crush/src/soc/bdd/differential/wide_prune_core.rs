use logging::PruneLogger;

use super::*;

// use logging::*;


impl Bdd {



    // fixme, improve on doc. What is an "arena", how to better explain "weight threshold", explain
    // "complexity"?. What to explain here, at what to explain at the crate diff level doc?
    /// Will continue to prune the given `shard` until the complexity of the `shard` has decreased
    /// **beneath** the desired complexity (`complexity_target`) or, if given, the optional weight
    /// bound (`weight_retain`) has been reached; whichever comes first.
    ///
    /// Returns the `arena` containing the `Id`'s of the deleted nodes, the level they ware found at,
    /// and also the nodes' corresponding trail's weights.
    ///
    /// ### Process:
    /// The method works by finding the *highest* `weight threshold` where at least one node will be
    /// pruned away, and then prunes away all nodes which contains only trails of weight
    /// **higher than** the `weight threshold`. Repeat the process until `self` is below the given
    /// `complexity_target`.
    /// TODO FIXME update
    /// This method will return early if:
    /// 1) The `weight threshold` indicates that the trivial solution passes through all the
    /// remaining nodes. With the current implementation of this method, that means that either *all*
    /// the remaining nodes would be pruned, or we must settle for none.
    ///
    /// 2) A floor limit for the `weight threshold` was given, and we reach this floor before we
    /// reach the `complexity target`. Such a floor may be given if it is undesirable to look for
    /// trails beneath a certain weight.
    ///
    // Prunes until the complexity of the Shard is less than the given threshold or weight_threshold
    // FIXME this is the doc for the v1 of this pruning strategy!
    pub fn complexity_based_wide_prune_v2<R, L: PruneLogger> (&mut self,
                                                           complexity_target: usize,
                                                           working_area: R,
                                                           step: usize,
                                                           librarian: &mut L,
    )
        where
            R: RangeBounds<usize>
    {
        let active_area = self.process_range(working_area, step);
        // Setup for logging
        let mut prune_logger = PruneRecordBuilder::new(step, self.get_size(), complexity_target, active_area.clone(), 2);

        while self.get_size() > complexity_target {
            // Identify widest level that is within the active area. If tie, current tiebreaker is
            // to get the lower level (lower level has a higher depth value). This tiebreaker is
            // chosen due to the lower levels presumably being "fresher", and probably less pruned
            // already.
            //(Note that this comment may become outdated if the method changes w/o updating this comment).
            let (widest, s_widest) = self.widest_levels(&active_area);
            // Decide the upper limit of how many nodes on that level to be marked for deletion.
            let roof_marked = Self::upper_limit_of_marked(widest.1, s_widest.1);

            // Calculate weights for the nodes at the widest level. Remember that the level may not
            // be a Centurion.
            let (arena, cohort_range) = self.ensure_level_is_in_arena(&widest.0, &active_area, step);

            // Find the highest LSB of that level. This LSB is then used as the threshold for which
            // nodes with that weight will be marked for deletion, up 'til the upper limit of marking
            let lsb = arena.highest_lsb_in_level(&widest.0).0;
            let mask = 2_u128.pow(lsb) - 1;

            // Mark all nodes with weight above threshold.
            let delete: Vec<Id> =
                arena.get_level(widest.0).expect("Using an outdated arena?")
                    .iter()
                    .filter(|(id, weight)| {
                        **weight & mask == 0
                    })
                    .take(roof_marked)
                    .map(|(id, _)| *id)
                    .collect();

            // Run the deletion algorithm
            let mut loop_logger =
                prune_logger.new_prune_loop_rec_builder(widest.clone(), s_widest.clone(),
                                                        cohort_range,
                                                        lsb, roof_marked, delete.len());

            self.delete_nodes_from_level_until(complexity_target, delete, widest.0, step, &mut loop_logger);
            prune_logger.register_loop_rec(loop_logger.finalize(self.get_size()));

            //Done
        }

        librarian.record(
            prune_logger.finalize(self.get_size())
        );
    }

    /// Find and return the depth of the widest and second widest level within the given depth range.
    /// **Returns** two tuples, where the first is the depth of the widest, along with its number of
    /// nodes, and the second tuple is the same, but for the second widest level.
    ///
    /// If several levels are tied, then the "deepest" level is chosen as 'widest', and the second
    /// "deepest" is chosen as 'second_widest'. This reasoning is built upon the assumption that new
    /// cohorts are added two the bottom of the Master shard, and thus the deeper levels will be
    /// "fresher" and less pruned. Hopefully you can look up the details behind why this is a good
    /// choice in a paper soon. Keep an eye on the readme and the module-level documentation for any
    /// updates.
    ///
    /// Will panic if self contains less than two levels. (As we need at least two levels in order
    /// to find the two widest...).
    pub fn widest_levels(&self, active_area: &Range<usize>) -> ((Depth, usize), (Depth, usize)) {
        assert!(self.get_levels_size() > 2, "Cannot find the two widest levels if we have less than two levels present.");
        #[cfg(debug_assertions)]
        let mut debug = vec![];

        // Width as key, level-depth as value(s)
        let mut all_widths = BTreeMap::new();

        // Get the widths of all the levels in 'active_area'.
        // Abut the use of rev(): Needed to start from the bottom in an earlier version of this
        // code. Not needed any more, but left in as its not a problem either, and more work to
        // change it than leave it.
        {
            // FIXME this is a simplified way of iterating over the given active area. As simpler
            // often is better (less likely to make errors, easier to understand, maintainability, etc),
            // it should be implemented. But that requires time to do correctly and to make sure that
            // nothing gets broken. Therefore it has to wait.
            // let offset = active_area.start;
            // for (a, l) in self.levels[active_area.start..active_area.end].iter().rev().enumerate() { println!("test,"); }
        }
        for (ascended, level) in self.iter_levels().rev().enumerate()
            // Skip sink and everything below the end of the active area
            .skip(self.get_levels_size() - active_area.end)
            // Skip everything above the end of the active area
            .take(active_area.end - active_area.start)
        {
            #[cfg(debug_assertions)]
            debug.push(active_area.end - ascended);

            let size = level.get_nodes_len();
            let levels = all_widths.entry(size).or_insert(Vec::new());
            levels.push(active_area.end - ascended);

        }

        // Get the size of the widest level(s), and the(ir) depth(s). Sort, to ensure that, in case
        // of a tie, the "deepest" level is always picked as 'widest'.
        let (size, candidates) = all_widths.iter_mut().last().unwrap();
        candidates.sort();

        let widest;
        let s_widest;


        if candidates.len() > 1 {
            widest = (candidates.pop().unwrap(), *size);
            s_widest = (candidates.pop().unwrap(), *size);
        } else {
            widest = (candidates.pop().unwrap(), *size);
            let (sw_size, sw_candidates) =
                all_widths.iter_mut().rev().skip(1).next().unwrap();

            sw_candidates.sort();
            s_widest = (sw_candidates.pop().unwrap(), *sw_size);
        }


        // Ensure that what we've found is inside the active area:
        #[cfg(debug_assertions)]
            {
                assert!(widest.0 >= active_area.start,
                        "Not within active area! Found widest at: {}. Active area: {}..{}\n\
                      Started iteration at: {}, ended at: {}. Nr of iterations: {}",
                                widest.0, active_area.start, active_area.end,
                                debug.first().unwrap(), debug.last().unwrap(), debug.len());
                // #[cfg(debug_assertions)]
                assert!(widest.0 < active_area.end, // fixme, <=
                        "Not within active area! Found widest at: {}. Active area: {}..{}\n\
                      Started iteration at: {}, ended at: {}. Nr of iterations: {}",
                                widest.0, active_area.start, active_area.end,
                                debug.first().unwrap(), debug.last().unwrap(), debug.len());
                // #[cfg(debug_assertions)]
                assert!(s_widest.0 >= active_area.start,
                        "Not within active area! Found widest at: {}. Active area: {}..{}\n\
                      Started iteration at: {}, ended at: {}. Nr of iterations: {}",
                                widest.0, active_area.start, active_area.end,
                                debug.first().unwrap(), debug.last().unwrap(), debug.len());
                // #[cfg(debug_assertions)]
                assert!(s_widest.0 < active_area.end,
                        "Not within active area! Found widest at: {}. Active area: {}..{}\n\
                      Started iteration at: {}, ended at: {}. Nr of iterations: {}",
                                widest.0, active_area.start, active_area.end,
                                debug.first().unwrap(), debug.last().unwrap(), debug.len());
            }

        // Sanity check!
        //FIXME improve to cover non active area as well!
        // let narrowest = all_widths.get(&1);
        // if narrowest.is_some() {
        //     panic!("At least one more level than source and sink have only one node! {:#?}", narrowest.unwrap());
        // }

        (widest, s_widest)

    }

    /// Calculates a roof for how many nodes which we will allow to be marked for deletion for this
    /// round of deletions.
    /// It is calculated as the difference (in terms of nodes) between the widest the
    /// s_widest (second_widest), plus a percentage of s_widest.
    ///
    /// The percentage is currently hardcoded to be 10%.
    pub fn upper_limit_of_marked(widest: usize, s_widest: usize) -> usize {
        let percentage = 10;
        let part = ((s_widest * percentage) as f64 / 100_f64) as usize;

        // Width difference + percentage of s_widest width
        widest - s_widest + part
    }


    /// As the weights in a level are dependent on the levels below it, we know that we need to fill
    /// a NodeWeightArena in order to get the right values.
    /// The standard way will only calculate the values for a Cohort's 'Centurion level', as that is
    /// all the Centurion of the above adjacent Cohort needs to know in order to calculate its own
    /// values. However, the widest level of a Shard is not always a Centurion.
    ///
    /// *This method ensures that the given level is part of the returned arena, whether it is a
    /// Centurion of only a member of the relevant Cohort.*
    ///
    /// Returns a NWArena and the range of the Cohort where 'member' is a member. (What depths that
    /// cohort lies).
    ///
    /// Please see 'identify_trails_and_weights_core()' for an overview of what invariants are
    /// expected to be upheld.
    pub fn ensure_level_is_in_arena(&self, member: &Depth, active_area: &Range<usize>, step: usize) -> (NWArena, Range<usize>) {
        // Setup, figure out which Cohort this level is a member of, and the range of this
        // Cohort. (cohort range :> c_depth..p_depth ).

        // Depth of member
        let m_depth = *member;

        let mut i = active_area.start + step;
        // Previous Centurion's depth
        let p_depth = loop {
            if m_depth < i { break i ;}
            if i > active_area.end {
                panic!("Unable to identify the end bound for the Cohort for this level: {}", member)
            };
            i += step;
        };

        // Member's Centurion's depth =  start bound for the Cohort of the given level
        let c_depth = p_depth - step;

        // Return arena based upon the given levels status in the cohort:
        if m_depth == c_depth {
            // If the given level is a Centurion, things are more straightforward: build and return
            // an NWArena as usual.
            // sub_range, b/c we don't want to do more work than we have to: Stop when this Centurion
            // knows its weights.
            let sub_range = Range{start: c_depth, end: active_area.end };
            (self.identify_trails_and_weights_core(&sub_range, step), c_depth..p_depth)

        } else {
            // Else, we need to calculate weights for the member level, but we don't need to know
            // its Centurions weights.
            // sub_range, b/c we don't want to do more work than we have to: Start at this cohorts end
            // as we don't need this Centurion weights.
            let sub_range = Range{start: p_depth, end: active_area.end };
            let mut arena = self.identify_trails_and_weights_core(&sub_range, step);
            arena.insert_level(self.identify_trails_and_weights_for_member_level(m_depth,
                                                                                 c_depth,
                                                                                 (p_depth, arena.get_level(p_depth).unwrap())
            ), m_depth);
            (arena, c_depth..p_depth)
        }


    }

}