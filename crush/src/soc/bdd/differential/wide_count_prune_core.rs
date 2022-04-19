use logging::PruneLogger;
use w_arenas::PathCount;

use super::*;

pub trait PPFactory {
    type ProgressBar: StyledProgressBar;

    fn new_progress_bar(&self, len: u64) -> Self::ProgressBar;
}

/// A trait allowing for flexibility for the user to choose what underlying ProgressBar they want to
/// use. In essence, this trait is almost like a newtype over any ProgressBar, with only the calls
/// to update and manipulate stat present. (I.e. all styling etc must be handled by the user when
/// implementing the Trait).
pub trait StyledProgressBar: Clone + Send {
    fn inc(&self, delta: u64);
    fn set_message(&self, msg: &str);
    fn finish_with_message(&self, msg: &str);
    fn finish_and_clear(&self);
    fn println(&self, msg: &str);
}

impl Bdd {
    pub fn complexity_based_wide_prune_v3<R, L, F>(&mut self,
                                                   complexity_target: usize,
                                                   working_area: R,
                                                   step: usize,
                                                   librarian: &mut L,
                                                   progress: F,

    )
        where
            R: RangeBounds<usize>,
            L: PruneLogger,
            F: StyledProgressBar,
    {
        let active_area = self.process_range(working_area, step);
        // Setup for logging
        let mut prune_logger = PruneRecordBuilder::new(step,
                                                       self.get_size(),
                                                       complexity_target,
                                                       active_area.clone(),
                                                       3);

        while self.get_size() > complexity_target {
            progress.set_message("Pruning");

            // Identify widest level that is within the active area. If tie, current tiebreaker is
            // to get the lower level (lower level has a higher depth value). This tiebreaker is
            // chosen due to the lower levels presumably being "fresher", and probably less pruned
            // already.
            //(Note that this comment may become outdated if the method changes w/o updating this comment).
            let (widest, s_widest) = self.widest_levels(&active_area);
            // Decide the upper limit of how many nodes on that level to be marked for deletion.
            let roof_marked = Self::upper_limit_of_marked(widest.1, s_widest.1);

            progress.set_message("Pruning: Preparing to delete");

            // Calculate weights for the nodes at the widest level. Remember that the level may not
            // be a Centurion.
            let (pwc_level, cohort_range) = self.count_at_level(&widest.0, &active_area, step);
            // Find the highest LSB of that level. This LSB is then used as the threshold for which
            // nodes with that weight will be marked for deletion, up 'til the upper limit of marking
            let lew = *pwc_level.lowest_existing_weights().iter().last().unwrap().0;

            let marked = pwc_level.iter()
                .filter(|(id, count)| {
                    count.lowest_non_zero_weight() >= lew
                })
                .map(|(id, count)| (id, count))
                .collect::<HashMap<&Id, &PWCount, BuildHasherDefault<AHasher>>>();

            let mut sorted_marked: BTreeMap<PathCount, Vec<&Id>> = BTreeMap::new();
            for (id, pwc) in marked.iter() {
                let sum = pwc.sum_trails();
                let ids = sorted_marked.entry(sum).or_insert(Vec::new());
                ids.push(*id);
            }
            // start marking with lowest sum
            let delete: Vec<Id> = sorted_marked.iter()
                .flat_map(|(_, ids)| ids.iter())
                .take(roof_marked)
                .map(|id| **id)
                .collect();

            // Run the deletion algorithm
            let mut loop_logger =
                prune_logger.new_prune_loop_rec_builder(widest.clone(), s_widest.clone(),
                                                        cohort_range,
                                                        lew, roof_marked, delete.len());
            let size_before = self.get_size();
            progress.set_message("Pruning: Deleting nodes");
            self.delete_nodes_from_level_until(complexity_target, delete, widest.0, step, &mut loop_logger);
            let size_after = self.get_size();
            assert!(size_after < size_before, "No nodes were deleted, we risk an infinite loop now, aborting!");
            progress.inc((size_before - size_after) as u64);
            prune_logger.register_loop_rec(loop_logger.finalize(self.get_size()));

            //Done
        }
        progress.finish_and_clear();
        librarian.record(
            prune_logger.finalize(self.get_size())
        );
    }

    pub fn count_at_level(&self, member: &Depth, active_area: &Range<usize>, step: usize) -> (PWCArenaLevel, Range<usize>) {
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
            (self.count_trails_and_weights_core(&sub_range, step), c_depth..p_depth)

        } else {
            // Else, we need to calculate weights for the member level, but we don't need to know
            // its Centurions weights.
            // sub_range, b/c we don't want to do more work than we have to: Start at this cohorts end
            // as we don't need this Centurion weights.
            let sub_range = Range{start: p_depth, end: active_area.end };
            let previous_centurion =
                self.count_trails_and_weights_core(&sub_range, step);

            (self.count_trails_and_weights_for_member_level(m_depth,
                                                            c_depth,
                                                            (p_depth, &previous_centurion)),
             c_depth..p_depth)

        }


    }
}