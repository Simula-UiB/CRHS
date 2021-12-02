use std::iter::FromIterator;
use std::num::NonZeroUsize;
use std::sync::{Arc};
use std::sync::mpsc::{SyncSender};

use vob::Vob;

use crush::soc::bdd::Bdd as Shard;
use crush::soc::bdd::differential::{DepPathFinder, Depth};
use crush::soc::bdd::differential::wd::{Node2NodeDistribution, WDArena, NcWDistribution};
use crush::soc::Id;
use crate::diff_solver::post_processing_v5::utils::path::Path;
use std::rc::Rc;
use std::cell::Cell;

/// Depth First Extraction of paths.
/// This one is intended to extracts paths which ends in a given target node
pub(crate) struct TargetedDFE<W> {
    /// Id of target node, the node which is our end point for paths
    target: Id,
    /// Depth which target node will be at
    target_depth: Depth,
    arena: Arc<WDArena<W>>,
    master: Arc<Shard>,
    step: NonZeroUsize,
    tx: SyncSender<Path>,
}

impl<W: Node2NodeDistribution> TargetedDFE<W> {
    pub(crate) fn new((target, target_depth): (Id, Depth),
                      arena: Arc<WDArena<W>>,
                      master: Arc<Shard>,
                      step: NonZeroUsize,
                      tx: SyncSender<Path>,
    ) -> TargetedDFE<W> {
        Self {
            target,
            target_depth,
            arena,
            master,
            step,
            tx,
        }
    }

    /// Depth First Extraction of paths. All paths extracted will *start at 'start node' and end at 'target'*,
    /// and also have their weight == "target weight'. ('target' was given as part of Self::new()).
    /// The 'upper limit' is the upper limit of paths extracted, and the process will quit either
    /// when all paths are exhausted, or if the number of paths extracted reach the upper limit.
    /// The upper limit is inclusive.
    ///
    /// This fn can run in concurrent mode. However, it will not coordinate with other threads. This
    /// implies that running this fn with the same parameters concurrently will yield all the same
    /// paths, not a subset of paths for each thread. It strength therefore lies in extracting paths
    /// of different weights concurrently.
    ///
    /// WARNING! Current impl does not support target depth being a member depth. Only "Centurions"
    /// are checked! FIXME
    pub(crate) fn extract_paths_targeted(&mut self,
                                         (start_node, start_depth): (Id, Depth),
                                         target_weight: u32,
                                         upper_limit: usize,
    ) -> ExtractionResult {
        let mut empty_vob = Vob::with_capacity(self.target_depth - start_depth);
        let extracted_count = Rc::new(Cell::new(0));

        self.extract_paths_targeted_core(&start_node,
                                         start_depth,
                                         target_weight,
                                         &mut empty_vob,
                                         upper_limit,
                                         extracted_count,
        )
    }





    fn extract_paths_targeted_core(&mut self,
                                   at_node: &Id,
                                   at_depth: usize,
                                   target_weight: u32,
                                   part_path: &mut Vob,
                                   upper_limit: usize,
                                   mut extracted_count: Rc<Cell<usize>>,
    ) -> ExtractionResult {
        // *Base case further down*

        // Find the connections one 'step' down
        let deps = DepPathFinder::new(*at_node, at_depth, self.step, self.master.as_ref());
        // Iterate over the connections, we will only choose the ones which will lead us closer to
        // 'target' but also only if the path will have target weight.
        for (child_id, child_path) in deps.into_iter() {
            let edge = child_path.iter().fold(false, |acc, b| acc | b);
            // Adjust target weight if we traversed at least one 1-edge
            let tgt_weight = if edge {
                target_weight - 1
            } else {
                target_weight
            };

            // Update depth
            // FIXME this update allows for only centurions to be checked, and not member levels
            // todo use levels in arena to get next depth? (OBS, depth in WDLevels are Options, and
            // that needs to be handled somehow).
            let child_depth = at_depth + self.step.get();

            // Base case: We've reached target depth
            if child_depth == self.target_depth {

                // The active area is non-inclusive, so we have to assume that the arena end also
                // is non inclusive. That means that we have to assume that any end nodes for the
                // path we're targeting is NOT in the arena. We therefore check child_id against
                // target_id here, instead of failing a child_id lookup prior to any recursive call.
                if child_id == self.target {
                    // Check that the path has the correct weight (tgt_weight == 0 if correct weight).
                    if tgt_weight != 0 {
                        continue;
                    }

                    // Send the path
                    part_path.extend_from_vob(&Vob::from_iter(child_path));
                    let _ = self.tx.send(part_path.into());
                    part_path.truncate(part_path.len() - self.step.get());

                    // Update count and then check towards limit
                    let c = Rc::get_mut(&mut extracted_count).unwrap();
                    *c.get_mut() += 1;
                    // The limit is reached and we're not to extract any more paths
                    if extracted_count.get() == upper_limit {
                        return ExtractionResult::LimitReached;
                    }
                    // else, we continue
                    continue;
                }

                // Second part of base case check: The child node was not the target, but since we're
                // still at the target depth, we should not keep going downwards. The target depth IS
                // the base case.
                continue;
            }

            // Continuing down the DAG:

            // Find child distribution in the arena:
            let child_dist = self.arena.get(&child_depth)
                .expect(&format!("Level not present in arena. Are you using an outdated arena? Depth: {}", child_depth))
                .get(&child_id)
                .expect(&format!("Expected to find a child distribution, found none! Child Id: {}. Depth {}", child_id, child_depth))
                .clone();

            // First, checking if the target_id is present in the distribution. If not, then
            // this node does not lead us towards the target node.
            // For paths_for_weight_in_id to return Some(), both the id must be present, and the
            // weight must have a non-zero count, i.e. paths of that weight must be present.
            if let Some(paths) = child_dist.paths_for_weight_in_id(tgt_weight, &self.target) {
                // Then checking if the target weight is possible to achieve through this node,
                // (looking back at only the EndNode's impl of paths_for_weight_in_id, I'm unsure
                // if we can trust that we cannot get a 0 back, so we check).
                if paths != &0 {
                    // FIXME ensure the right endianness!
                    part_path.extend_from_vob(&Vob::from_iter(child_path));

                    match self.extract_paths_targeted_core(&child_id,
                                                           child_depth,
                                                           tgt_weight,
                                                           part_path,
                                                           upper_limit,
                                                           extracted_count.clone()) {
                        ExtractionResult::LimitReached => {
                            return ExtractionResult::LimitReached;
                        },
                        ExtractionResult::LimitNotReached => {
                            // Continue
                        },
                    }

                    part_path.truncate(part_path.len() - self.step.get());
                }
            }
            // Else, continue;
        }

        ExtractionResult::LimitNotReached
    }



}


pub enum ExtractionResult {
    LimitReached,
    LimitNotReached,
}



/// Depth First Extraction of paths.
/// This one is intended to extracts paths which ends in a given target node
pub(crate) struct SemiTargetedDFE<W> {
    /// Depth which target node will be at
    target_depth: Depth,
    arena: Arc<WDArena<W>>,
    master: Arc<Shard>,
    step: NonZeroUsize,
    tx: SyncSender<Path>,
}

impl<W: NcWDistribution> SemiTargetedDFE<W> {
    pub(crate) fn new(target_depth: Depth,
                      arena: Arc<WDArena<W>>,
                      master: Arc<Shard>,
                      step: NonZeroUsize,
                      tx: SyncSender<Path>,
    ) -> SemiTargetedDFE<W> {
        Self {
            target_depth,
            arena,
            master,
            step,
            tx,
        }
    }

    /// Depth First Extraction of paths. All paths extracted will start at 'start node', *but may end*
   /// *in __any__ node at 'target depth'*. All extracted paths will have their weight == "target weight'.
   /// ('target' was given as part of Self::new()).
   /// The 'upper limit' is the upper limit of paths extracted, and the process will quit either
   /// when all paths are exhausted, or if the number of paths extracted reach the upper limit.
   /// The upper limit is inclusive.
   ///
   /// This fn can run in concurrent mode. However, it will not coordinate with other threads. This
   /// implies that running this fn with the same parameters concurrently will yield all the same
   /// paths, not a subset of paths for each thread. It strength therefore lies in extracting paths
   /// of different weights concurrently.
   ///
   /// WARNING! Current impl does not support target depth being a member depth. Only "Centurions"
   /// are checked! FIXME
    pub fn extract_paths_semi_targeted(&mut self,
                                       (start_node, start_depth): (Id, Depth),
                                       target_weight: u32,
                                       upper_limit: usize,
    ) -> ExtractionResult {
        let mut empty_vob = Vob::with_capacity(self.target_depth - start_depth);
        let extracted_count = Rc::new(Cell::new(0));

        self.extract_paths_semi_targeted_core(&start_node,
                                              start_depth,
                                              target_weight,
                                              &mut empty_vob,
                                              upper_limit,
                                              extracted_count,
        )
    }


    /// Will only target path weights, and not any end node
    /// OBS, levels checked are current depth + step, so no "member" or intermediate levels are
    /// checked. The step between levels are thus assumed to be constant.
    fn extract_paths_semi_targeted_core(&mut self,
                                        at_node: &Id,
                                        at_depth: usize,
                                        target_weight: u32,
                                        part_path: &mut Vob,
                                        upper_limit: usize,
                                        mut extracted_count: Rc<Cell<usize>>,
    ) -> ExtractionResult {
        // *Base case further down*

        // Find the connections one 'step' down
        let deps = DepPathFinder::new(*at_node, at_depth, self.step, self.master.as_ref());
        // Iterate over the connections, we will only choose the ones which will lead us closer to
        // 'target' but also only if the path will have target weight.
        for (child_id, child_path) in deps.into_iter() {
            let edge = child_path.iter().fold(false, |acc, b| acc | b);
            // Adjust target weight if we traversed at least one 1-edge
            let tgt_weight = if edge {
                target_weight - 1
            } else {
                target_weight
            };

            // Update depth
            // FIXME this update allows for only centurions to be checked, and not member levels
            // todo use levels in arena to get next depth? (OBS, depth in WDLevels are Options, and
            // that needs to be handled somehow).
            let child_depth = at_depth + self.step.get();

            // **** Base case: We've reached target depth ****
            if child_depth == self.target_depth {

                // The active area is non-inclusive, so we have to assume that the arena end also
                // is non inclusive. That means that we have to assume that any end nodes for the
                // path we're targeting is NOT in the arena. We therefore handle the base case
                // "one step too early", instead of failing a child_id lookup prior to any recursive call.

                // Check that the path has the correct weight (tgt_weight == 0 if correct weight).
                if tgt_weight != 0 {
                    continue;
                }

                // Send the path
                part_path.extend_from_vob(&Vob::from_iter(child_path));
                let _ = self.tx.send(part_path.into());
                part_path.truncate(part_path.len() - self.step.get());

                // Update count and then check towards limit
                *Rc::get_mut(&mut extracted_count).unwrap().get_mut() += 1;
                // The limit is reached and we're not to extract any more paths
                if extracted_count.get() == upper_limit {
                    return ExtractionResult::LimitReached;
                }
                // else, we continue
                continue;

            }

            // **** Non Base case: Continuing down the DAG ****

            // Find child distribution in the arena:
            let child_dist = self.arena.get(&child_depth)
                .expect(&format!("Level not present in arena. Are you using an outdated arena? Depth: {}", child_depth))
                .get(&child_id)
                .expect(&format!("Expected to find a child distribution, found none! Child Id: {}. Depth {}", child_id, child_depth))
                .clone();

            // Check if there is any paths of current weight in child, otherwise we continue
            if let Some(paths) = child_dist.paths_for_weight(tgt_weight) {
                // Then checking if the target weight is possible to achieve through this node,
                if paths != &0 {
                    // FIXME ensure the right endianness!
                    part_path.extend_from_vob(&Vob::from_iter(child_path));

                    match self.extract_paths_semi_targeted_core(&child_id,
                                                                child_depth,
                                                                tgt_weight,
                                                                part_path,
                                                                upper_limit,
                                                                extracted_count.clone()) {
                        ExtractionResult::LimitReached => {
                            return ExtractionResult::LimitReached;
                        },
                        ExtractionResult::LimitNotReached => {
                            // Continue
                        },
                    }

                    part_path.truncate(part_path.len() - self.step.get());
                }
            }
            // Else, continue;
        }

        ExtractionResult::LimitNotReached
    }
}