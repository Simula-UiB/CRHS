use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Result as FmtResult;
use std::sync::Arc;

use num_traits::ToPrimitive;

use crush::soc::bdd::differential::{PPFactory, StyledProgressBar};
use crush::soc::bdd::differential::wd::{EndNodeDist, Node2NodeDistribution, NWDistribution, PathCount, WDLevel, WDPresence, WDCountV2, NcWDistribution};
use crush::soc::Id;

/// There has been some confusion what "kind" of weight the sub_dist of a SessEstimate is.
/// This type is introduced to make it explicit and readily clear that it is the weight of "inner
/// paths" we record. I.e. of paths between a(n alpha) start node and an (beta) end node.
pub type InnerWeight = u32;

/// A struct capturing details about the estimated probability between a single start node to a
/// single end node (SESS).
/// See module level for details on how this is done? TODO
#[derive(Debug, Clone)]
pub struct SessEstimate {
    start: Id,
    end: Id,
    estimate: f64,
    sub_dist: BTreeMap<InnerWeight, PathCount>,
    beta_w: u32,
    /// The weight distribution between the start and end nodes. Expected to be *all* weights and
    /// of *all* paths between in the hull.
    hull_distribution: Option<WDCountV2>,
}

impl SessEstimate {
    /// Id of start node, aka the Single Start node
    #[inline]
    pub fn start(&self) -> Id {
        self.start
    }

    /// Id of end node, aka the Single End node
    #[inline]
    pub fn end(&self) -> Id {
        self.end
    }

    /// Estimated approximated probability for this SESS.
    #[inline]
    pub fn estimate(&self) -> f64 {
        self.estimate
    }

    /// Set the weight distribution between the start and end nodes. Expected to be *all* weights and
    /// of *all* paths between in the hull.
    #[inline]
    pub fn set_hull_distribution(&mut self, hull_dist: WDCountV2) {
        self.hull_distribution = Some(hull_dist);
    }

    /// The weight distribution between the start and end nodes. Expected to be *all* weights and
    /// of *all* paths between in the hull.
    pub fn hull_distribution(&self) -> Option<&WDCountV2> {
        self.hull_distribution.as_ref()
    }


    /// The distribution of paths and weights for which this SESS's estimate is based upon.
    /// The weights are based upon the Beta-Alpha arena.
    /// See comments inside fn "estimate_best_sess_connections" for details on how this dist is made.
    #[inline]
    pub fn dist(&self) -> &BTreeMap<InnerWeight, PathCount> {
        &self.sub_dist
    }

    /// The nt_lew of this SESS's 'dist'. This is the nt_lew of the "inner paths" between start node
    /// and end node.
    ///
    /// Assumes the SESS is not for the trivial path.
    #[inline]
    pub fn inner_nt_lew(&self) -> u32 {
        self.sub_dist.keys().next().cloned().unwrap()
    }

    #[inline]
    pub fn beta_nt_lew(&self) -> u32 {
        self.beta_w
    }

    /// Write a simple summary of self to 'buff'.
    pub fn fmt_log_entry(&self, f: &mut fmt::Formatter) -> FmtResult {
        // todo improve
        writeln!(f, "{:-^100}", " SESS Estimate ")?;
        writeln!(f, "Sess Estimate: Start node: {}. End node: {}. Estimate: {}",
                 self.start, self.end, self.estimate)?;
        writeln!(f, "Sub dist for inner paths: {:?}", self.sub_dist)?;
        writeln!(f, "Inner nt lew: {}", self.inner_nt_lew())?;
        writeln!(f, "Beta nt lew: {}", self.beta_nt_lew())?;

        if let Some(dist) = &self.hull_distribution {
            writeln!(f, "These are the number of active S-boxes and their number of paths, from the\n\
            (alpha) start node to the (beta) end node. Weights and counts are therefore not including the last round!")?;
            writeln!(f, "Observed weight | Number of times")?;
            for (w, c) in dist.existing_weights_with_counts() {
                writeln!(f, "{: >15} | {: >3}", w, c)?;
            }
        } else {
            writeln!(f, "Al[ha->Beta distribution not given!")?;
        }

        Ok(())
    }

    // todo update or remove
    /// Does what is says, but is intended for debug purposes.
    /// Warning, not up to date, as in some newer fields are not yet included.
    pub fn debug_compare(&self, other: &SessEstimate) -> bool {
        if self.start != other.start {
            println!("Start nodes are not equal");
            return false;
        }
        if self.end != other.end {
            println!("End nodes are not equal");
            return false;
        }
        if self.estimate*1000.0 != other.estimate*1000.0 {
            println!("{}",
                     format!("Estimates are not equal: self: {}, other: {}",
                             self.estimate, other.estimate));
            return false;
        }

        if self.dist() != other.dist() {
            println!("Dists are unequal");
            return false;
        }
        true
    }
}

pub enum DisplaySessEst<'a> {
    AsLog(&'a SessEstimate),
}

impl fmt::Display for DisplaySessEst<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        match self {
            DisplaySessEst::AsLog(est) => {est.fmt_log_entry(f)?},
        }

        Ok(())
    }
}




/// Searches for the alpha nodes whose nt_lew are contained in the weight range:
/// >  weight_range = alpha_lvl_nt_lew..alpha_lvl_nt_lew + 3
///
/// Returns a tuple, where first value is the alpha level nt lew, and the second value is a
/// vec of vecs, each inner vec containing nodes sorted by the node nt lew values.
/// Example:
/// >  index 0 contains the nodes with nt_lew == alpha_lvl_nt_lew.
/// >  index 1 contains the nodes with nt_lew == alpha_lvl_nt_lew + 1.
///
/// The vecs are empty iff no such nodes were found.
///
/// Consumes the TauAlphaDistribution Level.
pub fn alpha_candidates(tau_alpha_dists: WDLevel<WDPresence>) -> (u32, Vec<Vec<(Id, WDPresence)>>) {

    // Non-trivial lew for the Alpha level as a whole. Since this is from Tau to Alpha, this
    // implies that the level nt lew is the lowest existing non trivial path from one Alpha path
    // to the sink, and thus the overall fewest active S-boxes we've found.
    let lvl_nt_lew = match tau_alpha_dists.nt_lew() {
        Some(nt) => nt.0,
        None => panic!("We only have the trivial path present!"),
    };

    // Range of weights in which want to investigate further.
    let weight_range = lvl_nt_lew..lvl_nt_lew + 3;

    // Collect and return the alpha nodes which nt_lew is within the weight_range, sorted into vec's by weight
    let candidates = tau_alpha_dists.into_iter()
        .fold(vec![vec![], vec![], vec![]],
              |mut acc, dist| {
                  let nt_lew = dist.1.lowest_existing_non_trivial_weight();
                  // If does not have an nt_lew, then not a candidate
                  if nt_lew.is_none() {
                      return acc;
                  }
                  let nt_lew = nt_lew.unwrap();
                  // If the nt_lew is too large, then not a candidate
                  if !weight_range.contains(&nt_lew) {
                      return acc;
                  }
                  // We've found a candidate, add to vec holding candidates of same nt_lew.
                  acc[(nt_lew % lvl_nt_lew) as usize].push(dist);
                  acc
              }
        );
    // .filter(|(_, dist)| {
    //     let nt_lew = dist.lowest_existing_non_trivial_weight();
    //     if nt_lew.is_none() {
    //         return false;
    //     }
    //     weight_range.contains(&nt_lew.unwrap())
    // })
    //  .fold(vec![vec![], vec![], vec![]],
    //        |mut acc, dist| {
    //            let nt_lew = dist.1.lowest_existing_non_trivial_weight().unwrap();
    //            acc[(nt_lew % lvl_nt_lew) as usize].push(dist);
    //            acc
    //        }
    //  );

    (lvl_nt_lew, candidates)
}


pub fn estimate_best_sess_connections<P>(alpha_candidates: Vec<(Id, WDPresence)>,
                                     beta_level_dists: Arc<WDLevel<WDPresence>>,
                                     alpha_beta_dist: Arc<WDLevel<EndNodeDist>>,
                                     alpha_level_nt_lew: u32,
                                     // NT lew of alpha candidates in alpha_candidates: Vec
                                     candidates_nt_lew: u32,
                                     // How many sess cons max to return
                                     max_connections: usize,
                                     progress: &P,
                                     k: f64,
) -> Vec<SessEstimate>
    where
        P: PPFactory,
{
    if alpha_candidates.is_empty() {
        return vec![];
    }

    // Make sure that the number of SESS candidates does not grow into the sky, by setting an upper
    // limit. We set a limit either to 10% more than 'max_connection", or at 20000, whichever is
    // largest. The threshold for when to truncate the vec of SESS'es are set higher than the
    // max_connections in order to save work.
    let truncate_threshold = usize::max(20_000, (max_connections as f64 * 1.1) as usize);
    // About 'candidate_nt_lew':
    // NT lew of alpha candidates in alpha_candidates: Vec. Since alpha_candidates are alpha dists
    // from Tau to Alpha, this implies that the candidates nt lew is the lowest existing non trivial
    // path from one alpha candidate node to the sink, and thus the overall fewest active
    // S-boxes we've found starting in that node. If candidate nt lew == alpha level nt lew, then
    // this is the overall fewest number of active S-boxes for any trail.

    // Renamed for consistency with naming convention within this fn: Differs from argument name as
    // I feel that the argument name is more informative for users of this fn, while this rename is
    // more informative for readers of this code.
    let alpha_node_nt_lew = candidates_nt_lew;

    // Progress report over alpha nodes
    let progress_outer = progress.new_progress_bar(alpha_candidates.len() as u64);
    progress_outer.set_message(&format!("Searching... Target nt lew: {}", candidates_nt_lew));

    // Keep track of best SESS Con candidates. Will be reduced down to 'cutoff' once it passes 2000
    // entries. (2000 because is seems reasonably not too often, yet not too large).
    // 'Cutoff' of the best entries will eventually be returned.
    let mut sess_es = Vec::new();

    // We now iterate over the various start nodes in Alpha lvl. These nodes are all
    // considered the Start Node for MESS Connections, as they may have multiple end nodes.
    // (MESS = Multiple End-nodes, Single Start-node Connections. See mod level discussion?).
    // We are looking for the SESS Con's with the estimated best hull.
    // (SESS = Single End-node, Single Start-node Connection. One SESS may have multiple paths
    // going from start node to end node, thus we're talking about finding the best hull).
    for (alpha_start_id, alpha_start_dists) in alpha_candidates {
        // Keeping tabs on the progress over end nodes
        let progress_inner = progress.new_progress_bar(beta_level_dists.len() as u64);

        // If the MESS Con does not contain any paths of relevant weights, then it is not of
        // interest to us.
        // We therefore ensure that at least one such path exists, before we attempt to find
        // the relevant SESS Con or Con's:
        // For the alpha start node, check if at least one of the available end nodes contains
        // W, W+1 and/or W+2, where W is the alpha node nt-lew (=> W is expected to always be present,
        // but is "invariant" checked through this).
        let mut start_contains = Vec::with_capacity(3);
        let alpha_existing_weights = alpha_start_dists.existing_weights();

        for w in alpha_node_nt_lew..alpha_node_nt_lew + 3 {
            if alpha_existing_weights.contains(&w) {
                start_contains.push(w);
            }
        }

        // The MESS Con does not contain any paths of relevant weights, let's look at the next one.
        // (This one is kept as it documents nicely an important invariant. However, the way the code
        // is written right now should make it redundant (see above).)
        if start_contains.is_empty() {
            progress_outer.inc(1);
            continue;
        }


        // -----------------------------------------------------------------------------------

        // Now we have at least one SESS Con with path(s) of one or more relevant weights.
        // Time to find them.

        // For each Single End-node (SE),
        for (beta_id, beta_dist) in beta_level_dists.iter() {
            // check to see if this is (one of) the SESS Con(s) containing paths of relevant
            // weight(s).

            // The sub_dists keeps tab of W..W+3, and number of *inner* paths
            // The sub_dists needs to be init here, but leaving it as empty will allow us to
            // correctly init it once beta_w is know, or to "skip" it if beta_w is None
            let mut inner_sub_dist = BTreeMap::new();
            // the inner sub dist is used later to correctly identify inner paths for when we
            // calculate the actual probabilities for the best differential/hull.
            // alpha_sub_dist is used to calculate the estimated probability
            let mut alpha_sub_dist = BTreeMap::new();

            // beta_w is the weight for a path from the Beta level down to Tau (the sink).
            // We know that beta_w < w for all non-trivial weights. (beta_w == w implies that
            // all the active S-boxes are in the Beta path, meaning that the Alpha path must
            // have non active S-boxes.
            // This is only possible if the Alpha path is the trivial solution, which in turn
            // implies that the Beta path is also the trivial solution, thus beta_w == w is
            // only true iff w is 0).
            //
            // In theory, any beta_w such that 'beta_w + beta_alpha_w = w' is a viable beta_w,
            // and our initial research iterated over all these possible configurations to
            // see how many paths we could find. However, a different beta_w means that we
            // operate with different Beta paths, which in turn means that all the paths we
            // found were not for the same Beta characteristic, and thus they were for
            // different hulls.
            // We decided to only keep track of the lowest beta_w, since fewer active
            // S-boxes always gives a better probability than more active S-boxes. This holds
            // true even when the DDT is non-uniform. The reasoning behind this is too long
            // to put here, but hopefully it will be explained in the mod docs.
            //
            // To summarize, beta_w is the fewest active S-boxes possible for this Beta end-
            // node down to Tau. We use this to figure out how many active S-boxes there are
            // between this Beta SE and its corresponding Alpha Single Start-node, for the
            // path to have weight of w. Which in is in turn used to read from the relevant
            // count of paths from the Beta_alpha_dists.
            // If only the trivial beta_w is present, we continue with next Beta SE instead.
            // (For the same reasoning as to why beta_w != w).
            if let Some(beta_node_nt_lew) = beta_dist.lowest_existing_non_trivial_weight() {
                // The sub_dist needs to be initialized to "0 paths" for the relevant weights, so
                // that we may calculate the estimate later w/o problems.
                for w in alpha_node_nt_lew..alpha_node_nt_lew + 3 {
                    inner_sub_dist.insert(w - beta_node_nt_lew, 0);
                    alpha_sub_dist.insert(w, 0);
                }

                for w in start_contains.iter() {

                    // Now we do two things in one go:
                    // 1) Check if there exists a path between the Beta end-node and the
                    // Alpha start-node,
                    // 2) and if such a path exists, we get the number of paths for w - beta_w
                    // (= beta_alpha_w, aka weight for the path(s) between Beta node and Alpha node).\
                    let maybe_count = alpha_beta_dist
                        .get(beta_id).expect("Unexpected, the end node is missing!")
                        .paths_for_weight_in_id(w - beta_node_nt_lew, &alpha_start_id);

                    if let Some(count) = maybe_count {
                        // Store the alpha_beta_w with the associated count of paths.
                        // We store the alpha_beta_w instead of w, as alpha_beta_w is the
                        // target weight when we extract the relevant inner paths later.
                        let _ = inner_sub_dist.insert(w - beta_node_nt_lew, count.clone());
                        let _ = alpha_sub_dist.insert(*w, count.clone());
                    }
                }


                #[cfg(debug_assertions)]
                {
                    // We've checked all present w's for paths. If none were found, panic: This shouldn't be possible!
                    if inner_sub_dist.is_empty() || inner_sub_dist.values()
                        .fold(true, |acc, val| acc & (*val == 0))
                    {
                        panic!("Coming so far means that at least one path of weight \
alpha_node_nt_lew - beta_node_nt_lew should exist, but none were found!");
                    }
                }

                // We're gotten the count of all relevant paths for this SESS, time to register it:
                sess_es.push(SessEstimate {
                    start: alpha_start_id.clone(),
                    end: beta_id.clone(),
                    estimate: make_estimate(&alpha_sub_dist, k, alpha_level_nt_lew),
                    sub_dist: inner_sub_dist,
                    beta_w: beta_node_nt_lew,
                    hull_distribution: None,
                });
            }

            if sess_es.len() > truncate_threshold {
                sess_es.sort_unstable_by(|a, b| {
                    // OBS! We're comparing f64's here, something I am a bit queasy to do, but since
                    // I'm not looking for exact matches anyways, I think we're fine.
                    b.estimate.partial_cmp(&a.estimate).unwrap()
                });

                sess_es.truncate(max_connections);
            }
            progress_inner.inc(1);
        }
        progress_inner.finish_and_clear();

        // All SESSes starting from this Alpha start-node have now been checked and their
        // estimates calculated.
        sess_es.sort_unstable_by(|a, b| {
            // OBS! We're comparing f64's here, something I am a bit queasy to do, but since
            // I'm not looking for exact matches anyways, I think we're fine.
            b.estimate.partial_cmp(&a.estimate).unwrap()
        });

        sess_es.truncate(max_connections);
        progress_outer.inc(1);
    }

    // We're done!
    if sess_es.len() != 0 {
        progress_outer.finish_with_message("Found estimate(s)");
    } else {
        progress_outer.finish_with_message("Only the trivial path was found!");
    }

    sess_es.truncate(max_connections);
    sess_es
}



/// For the given weights and the corresponding count of paths with those weights, calculate the
/// estimated differential/hull probability of this alpha -> beta characteristic.
/// The given weights must be the weights for the complete (but not the extended) trail.
fn make_estimate(tau_alpha_sub_dist: &BTreeMap<u32, PathCount>, k: f64, alpha_level_nt_lew: u32) -> f64 {
    // Assuming that the weights are for the complete (but not extended) trail, then
    // the estimate will be calculated as
    //   count0 * 2^(-(weight0 - alpha__level__nt lew)*k)
    // + count1 * 2^(-(weight1 - alpha__level__nt lew)*k)
    // + etc...
    // This will allow us to compare apples with apples later on.

    tau_alpha_sub_dist.iter()
        .fold( 0.0,
            |mut acc, (weight, count)|
                {
                    let pow = 2_f64.powf
                    (
                        -k * (weight - alpha_level_nt_lew)
                            .to_f64().expect("Failed to convert count to f64")
                    ); //FIXME check how the estimatges are sorted later on!

                    acc += *count as f64 * pow;
                    acc
                }
        )
}