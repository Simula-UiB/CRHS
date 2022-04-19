

//! ## Invariant
//! This problem statement assumes that the DAG has already been reduced such that only the only node
//! left on the alpha level and beta level is the chosen alpha candidate and chosen beta beta
//! candidate, respectively. This means that all the only inner paths present are all part of the hull.
//!
//!
//! ## Problem statements/question:
//!
//! 1) How do we generate the Alpha path and the Beta path?
//!
//!     a) Extract one from the paths existing in the DAG
//!
//!     > i) Advantage: Will always match with all inner paths, i.e. all paths can be used when
//!             calculating the hull.
//!
//!     > ii) Disadvantage: The likelihood of picking at random an "optimal" alpha path, meaning
//!             a path were all the active S-boxes yields the "optimal" influence, is very low.
//!
//!     > iii) This disadvantage could be attempted mitigated by searching through all available
//!             alpha paths to see which one would be best. However, there is no guarantee that such
//!             an "optimal" path will be part of the set of alpha paths. In general, we do not expect
//!             it to be present. How likely/unlikely it is to be present varies from cipher to cipher.
//!
//!     b) Construct Alpha/Beta path based on an existing inner path, independently of whether or not
//!             the resulting alpha beta pair exists in the DAG or not.
//!
//!     > i) Advantage: Will always be "optimal" (see "optimal" above), as we choose the corresponding
//!             input for the given output from the inner path.
//!
//!     > ii) Disadvantage: May, and often will, not use all the inner paths of the hull, as only
//!             inner paths passing through the same "n-node" (a node on depth alpha+n, where n is the
//!             number of bits in the output of alpha S-box layer) will "fit" with this constructed Alpha.
//!             -> There is probably a similar mechanic going on for the constructed Beta path, but the
//!             details here are yet to be looked into.
//!
//!     > iii) This disadvantage may be somewhat mitigated by choosing the "n-node" with the
//!             most paths of (ideally) alpha-level nt-lew passing through it. (If the ratio of number
//!             of paths exceeds (insert eq here), then using alpha-level nt-lew + 1 will give better
//!             results).
//!
//!     > => c) When to use a) and when to use b) can be calculated using (insert eq here). However,
//!             as this code is currently a proof-of-concept, we've opted to calculate for both a) and b)
//!             every time. It keeps the code simpler, and we get more data.
//!             3) Path influence calculation vary by analysis.
//!             A path influence under Linear mode will need an extra multiplication by 2 at some point,
//!             before the various path influences are multiplied and added. This is needed in order to
//!             be in compliance with the "piling up lemma", as formulated by Matsui.
//!
//! 2) The number of paths in the CRHS equation may easily overflow an usize. In practice, they may
//!     be too many paths for us to include them all in the hull probability calculation. This
//!     necessitates that an upper limit of the number of paths we include is given, which in turn
//!     may (and will) influence how we deal with path extraction from the DAG. As a general rule,
//!     we wish to always include the most beneficial paths (i.e. the paths with the fewest number
//!     of S-boxes). From an efficiency point of view, this leaves us with two cases:
//!
//!      > a) First case: The total number of paths are less than the upper limit, and they can all
//!         be included in the hull calculation. This is the easiest case, as all the paths may be
//!         extracted from the DAG in an arbitrary order, where we only need to be careful not to
//!         extract the same path twice. Basic graph traversal will handle this with a minimum of
//!         overhead.
//!
//!     > b) Upper limit is reached/breached: We could traverse the paths in arbitrary order, but
//!         that would most likely yield a sub-optimal hull probability. Reasoning: We've observed
//!         that that for hulls large enough to warrant an upper boundary for paths traversed,
//!         (but not limited to only these hulls), there are more paths with a higher amount of active
//!         S-boxes than of fewer. This, in turn, means that we are more likely to pick a path
//!         with more active S-boxes than one with fewer. We therefore need to consider how we can
//!         ensure that we always start with the most "beneficial" paths.
//!
//!     > => Picking the paths with fewer active S-boxes necessitates that we use more metadata.
//!         We can achieve this by the use of a WDArena. As a consequence, reaching the upper limit
//!         will require more memory to handle than not reaching the limit. However, the WDArena may
//!         also be used to efficiently extract/construct "optimal" alpha/beta paths, as well as
//!         "optimal" example path(s).
//!
//! ### Observations:
//!    1) We have several different WD distributions too choose from when constructing a WDArena,
//!     where the distribution with the lowest memory footprint (at the time of writing) is the
//!     WDPresence. Choosing to build the WDArena using the WDPresence,and since we've already reduced
//!     the DAG (see invariant), the overall footprint of a WDArena can be expected to be within
//!     reasonable levels.
//!     Furthermore, as a WDArena may simplify the extraction of example paths as well as alpha/beta path
//!     extraction/construction, it may be a good idea to always construct a WDArena, independently
//!     of whether an upper limit is set or not.
//!
//!    However, using the WDArena for inner path extraction is probably something we'd want to use
//!     only for the "we've reached the upper limit" case, as it will add unnecessary overhead per
//!     path for the "upper limit not reached" case. (As mentioned, the not reached case does not
//!     require that the paths are extracted in a certain order, allowing for simpler techniques to be
//!     used).
//!
//!    2) All Sierra->Alpha paths ending in the alpha node will have the same number of active S-boxes,
//!     by necessity. This means that all the "n-nodes" on level alpha + n will have the same offset.
//!     In turn, this means that we can look directly at their numbers when looking for a path with alpha-level
//!     nt-lew, without the need to calculate and apply the offset. This n-level nt-lew will correspond
//!     to the alpha-level nt-lew, and thus be treated as such for the sake of differentiating paths.

// use std::sync::Arc;
// use crate::diff_solver::post_processing_v5::SolvedSocMeta;
// use crush::soc::bdd::{Bdd as Shard};
//
// mod priming;
// mod dfe;
// mod construct_alpha_beta;
//
// pub struct HullResult {
//
//
// }

// FIXME!!!!

// pub fn calculate_hull(master: Arc<Shard>, master_md: &SolvedSocMeta) {
//     // First thing first, priming
//     priming::make_primers(master.clone(), master_md);
//
//     // Second
// }


//! "top level" mod for hull calculation. We currently have two approaches to calculate the best
//! probability, and depending on circumstances, either may yield the best results.
//!
//! Fn's in this mod is intended for fn's used in both.
//! Need to modify extract paths fn's. For instance, yielding the end node's id may be a good ide to
//! include.
//!


use std::fmt;
use std::fmt::Result as FmtResult;

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, sync_channel, SyncSender};
use std::thread;

use vob::Vob;

use crush::algebra::Matrix;
use crush::soc::bdd::Bdd as Shard;
use crush::soc::bdd::differential::{Depth, PPFactory, StyledProgressBar};
use crush::soc::bdd::differential::wd::{NcWDistribution, TransparentFactory, WDCountV2, WDLevel};
use crush::soc::{Node, Id};
pub use results::{ProcessedResult, DisplayResult, ProcessedResultSection};

use crate::code_gen::SBoxHandler;
use crate::diff_solver::post_processing_v5::bt::bthandler_trait::BTHandler;
use crate::diff_solver::post_processing_v5::{Handlers, SolvedSocMeta};
use crate::diff_solver::post_processing_v5::sess_handling::SessEstimate;
use crate::diff_solver::post_processing_v5::utils::path::Path;
use crate::diff_solver::post_processing_v5::logging::Cache;
use crate::diff_solver::post_processing_v5::hull_calc_v2::dfe::SemiTargetedDFE;
use crate::diff_solver::post_processing_v5::hull_calc_v2::priming::Primed;

pub mod construct_alpha_beta;
pub mod extract_alpha_beta;
mod results;
mod dfe;
mod priming;

// Increments main_pb twice
pub fn calculate_hull<B,S, P>(master: Arc<Shard>,
                              cache: &Cache,
                              lhss: Matrix,
                              handlers: &Handlers<B,S>,
                              main_pb: &P::ProgressBar,
                              progress: &P,
                              generic_res: ResultSectionBuilder,

)
                              -> ProcessedResult
    where
        B: BTHandler,
        S: SBoxHandler,
        P: PPFactory,
{
    // Setup result builder for the extracted alpha path and beta path version
    let mut extract_result = generic_res.clone();
    extract_result.mode = BuildMode::Extracted;

    // Setup result builder for the constructed alpha path and beta path version
    let mut constructed_ab_result = generic_res;
    constructed_ab_result.mode = BuildMode::Constructed;

    //todo use multiple consumer.... Make concurrent?
    main_pb.set_message("Calc by create");
    construct_alpha_beta::calculate_hull(master.clone(),
                                         cache,
                                         &lhss,
                                         handlers,
                                         progress,
                                         &mut constructed_ab_result);
    cache.record_processed_result_section(constructed_ab_result.clone().into());
    main_pb.inc(1);

    main_pb.set_message("Calc by extract");
    extract_alpha_beta::calculate_hull(master,
                                       cache,
                                       lhss,
                                       handlers,
                                       progress,
                                       &mut extract_result,
    );
    cache.record_processed_result_section(extract_result.clone().into());
    main_pb.inc(1);

    ProcessedResult::new(vec![constructed_ab_result, extract_result])
}



#[derive(Debug, Clone)]
pub struct ResultSectionBuilder {
    mode: BuildMode,
    /// The actual SESS Estimate, with metadata
    best_estimate: SessEstimate,
    /// Number of paths in the hull
    paths_skipped: Option<usize>,
    /// The alpha path used
    alpha_path: Option<Path>,
    /// The beta path used
    beta_path: Option<Path>,
    /// An example path which will yield a best result. Should at least be a path which yields the
    /// fewest number of active S-boxes, but ideally one which also yields the lowest probability
    example_path: Option<Path>,
    /// The calculated probability for this hull
    hull_probability: Option<f64>,
    /// The various probability each path in the hull have, and how many paths have that probability
    /// OBS Keys are the actual probability/weight multiplied with a factor as given by the base table BT!
    probabilities_count: Option<BTreeMap<usize, usize>>,

    block_size: usize,
    num_rounds: usize,

    // /// A Shard may have more paths than a u64 can document, and thus it's not a good idea to always
    // /// calculate the weigths for all paths. This is the upper limit of paths used for this run-through.
    // upper_bound_paths_used: Option<usize>,
    // /// True iff not all paths in the Shard was used as there were too many paths. I.e. the upper limit
    // /// would've been breached.
    // upper_bound_paths_reached: bool,
}

impl ResultSectionBuilder {
    pub fn new(mode: BuildMode,
               best_estimate: SessEstimate,
               block_size: usize,
               num_rounds: usize,
    ) -> ResultSectionBuilder
    {
        Self {
            mode,
            best_estimate,
            paths_skipped: None,
            alpha_path: None,
            beta_path: None,
            example_path: None,
            hull_probability: None,
            probabilities_count: None,
            block_size,
            num_rounds,
            // upper_bound_paths_used: None,
            // upper_bound_paths_reached: false,
        }
    }
}


/// Was the Alpha/Beta paths yielded using the create alpha beta or the extract alpha beta approach?
#[derive(Debug, Clone)]
pub enum BuildMode {
    Template,
    Constructed,
    Extracted,
}

impl fmt::Display for BuildMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        use BuildMode::*;

        match self {
            Template => write!(f, "Unknown")?,
            Constructed => write!(f, "BuildMode: Alpha and Beta paths were constructed to be optimal")?,
            Extracted => write!(f, "BuildMode: Alpha and Beta paths were extracted from the graph")?,
        };

        Ok(())
    }
}



/// Extracts exactly one path from 'start_depth' to 'end_depth'. Calling this fn twice with the same
/// parameters may yield the same path twice, or it may yield two different paths. No guarantees are
/// given.
///
/// *INVARIANT* This method assumes that there is exactly *one* node on the start_depth level and
/// exactly *one* node on the end depth level.
fn extract_a_single_path(master: Arc<Shard>, start_depth: Depth, end_depth: Depth) -> Path {

    let start_nodes = master
        .level(start_depth)
        .expect("Start level is missing!")
        .get_nodes();
    assert_eq!(start_nodes.len(), 1);
    let mut current_node = start_nodes.values().next().expect("Start level is empty!");

    let mut current_depth: Depth = start_depth;
    let mut path = Vob::with_capacity(end_depth - start_depth);

    loop {
        if current_depth == end_depth {
            break path.into();
        }
        let child_depth = current_depth + 1;

        // Perhaps add an rand call of sorts, to vary which edge we try first? Any alpha path ending
        // in the same alpha node will have the same weight, but this implementation will somewhat
        // favour the ones with late active S-boxes.
        if let Some(e0) = current_node.get_e0() {
            path.push(false);
            let e0 = master
                .level(child_depth).expect("Missing a level")
                .get_node(&e0)
                .expect(&format!("Hit an unexpected dead end! Child depth: {}. End depth: {}",
                                 child_depth, end_depth));
            current_node = e0;
            current_depth = child_depth;
            continue;
        }
        if let Some(e1) = current_node.get_e1() {
            path.push(true);
            let e1 = master
                .level(child_depth).expect("Missing a level")
                .get_node(&e1)
                .expect(&format!("Hit an unexpected dead end! Child depth: {}. End depth: {}",
                                 child_depth, end_depth));
            current_node = e1;
            current_depth = child_depth;
            continue;
        }
    }
}

pub (crate) fn extract_limited_pahts_concurrently(master: Arc<Shard>,
                                                  (start_node, start_depth): (Id, Depth),
                                                  end_depth: Depth,
                                                  sender_bound: usize,
                                                  cahce: &Cache,
                                                  upper_limit: usize,
                                                  primed: Primed,
) {

    let (tx, rx) = sync_channel(sender_bound);

    let mut dfe = SemiTargetedDFE::new(
        cache.master_md().beta_lvl_depth,
        primed.arena.clone(),
        master,
        cahce.master_md().step.clone(),
        tx);

    {
        let tx = tx.clone();

        let _handle = thread::spawn(move || {
            dfe.extract_paths_semi_targeted((start_node, start_depth), )
        };

    }
}

/// Extract all the paths from 'start_depth' to 'end_depth', returning them through the given
/// sync_channel's sender. Each path will be yielded exactly once, and no paths are to be skipped.
///
/// *INVARIANT* This method assumes that there is exactly *one* node on the start_depth level and
/// exactly *one* node on the end depth level.
pub(crate) fn extract_all_paths_concurrently<P>(master: Arc<Shard>,
                                                start_depth: Depth,
                                                end_depth: Depth,
                                                sender_bound: usize,
                                                _pb: P::ProgressBar,
) -> (SyncSender<Path>, Receiver<Path>)
    where P: PPFactory,
{
    let (tx, rx) = sync_channel(sender_bound);

    let start_nodes = master
        .level(start_depth)
        .expect("Start level is missing!")
        .get_nodes();
    assert_eq!(start_nodes.len(), 1);

    let start_node = start_nodes.values().next().expect("Start level is empty!").clone();
    let mut path = Vob::new();

    {
        let tx = tx.clone();

        let _handle = thread::spawn(move || {
            // FIXME complains about missing lifetime bound, but I have no idea where it wants me to add it...
            // pb.set_message("Extracting inner paths...");
            extract_all_paths_core(master.clone(),
                                   &mut path,
                                   &start_node,
                                   start_depth,
                                   end_depth,
                                   &tx,
                                   // pb);
            );
            // pb.finish_and_clear();
        });
    }
    // Return the tx and rx, to use by whatever sees fit
    (tx, rx)
}

fn extract_all_paths_core(master: Arc<Shard>,
                          path: &mut Vob,
                          current_node: &Node,
                          current_depth: Depth,
                          end_depth: Depth,
                          tx: &SyncSender<Path>,
                          // mut pb: P::ProgressBar,
)
// ) -> P::ProgressBar
//     where P: PPFactory
{
    // Base case
    if current_depth == end_depth {
        let _ = tx.send(Path::from(path));
        // pb.inc(1);
        return;
        // return pb;
    }

    // main recursion
    let child_depth = current_depth + 1;

    if let Some(e0) = current_node.get_e0() {
        path.push(false);
        let e0 = master
            .level(child_depth).expect("Missing a level")
            .get_node(&e0)
            .expect(&format!("Hit an unexpected dead end! Child depth: {}. End depth: {}",
                             child_depth, end_depth));
        // pb = extract_all_paths_core(master.clone(), path, e0, child_depth, end_depth, tx, pb);
        extract_all_paths_core(master.clone(), path, e0, child_depth, end_depth, tx);
        path.pop();
    }

    if let Some(e1) = current_node.get_e1() {
        path.push(true);
        let e1 = master
            .level(child_depth).expect("Missing a level")
            .get_node(&e1)
            .expect(&format!("Hit an unexpected dead end! Child depth: {}. End depth: {}",
                             child_depth, end_depth));
        // pb = extract_all_paths_core(master.clone(), path, e1, child_depth, end_depth, tx, pb);
        extract_all_paths_core(master.clone(), path, e1, child_depth, end_depth, tx);
        path.pop();
    }
    // pb
}

/// Returns the number of alpha paths and beta paths ending/starting in the SESS Estimate's
/// alpha and beta node, respectively.
///
/// **Invariants**
/// * Only the SESS start node is left on the Alpha level
/// * Only the SESS end node is left in the Beta level
pub(crate) fn count_alpha_and_beta_paths_for_sess<P>(master: Arc<Shard>, master_md: &SolvedSocMeta, progress: &P)
                                                     -> (usize, usize)
    where P: PPFactory,
{
    // todo Logg
    let top_active_area = Range{start: 0, end: master_md.alpha_lvl_depth+1};
    let sierra_alpha: WDLevel<WDCountV2> = master.weight_distributions_for_level_top_bottom(
        master_md.alpha_lvl_depth,
        &top_active_area,
        master_md.step.clone(),
        &TransparentFactory::new(),
        progress,
    );
    debug_assert_eq!(sierra_alpha.len(), 1);
    let res_sa = sierra_alpha.iter().next().unwrap().1.total_number_of_paths_overflowing();
    if res_sa.1  {
        panic!("Overflow occurred while counting active S-boxes in Sierra->Alpha");
    }
    let sum_alpha_paths: usize = res_sa.0;

    let tau_beta: WDLevel<WDCountV2> = master.weight_distributions_for_level_top_bottom(
        master_md.alpha_lvl_depth,
        &master_md.active_area,
        master_md.step.clone(),
        &TransparentFactory::new(),
        progress,
    );
    debug_assert_eq!(tau_beta.len(), 1);
    let res_tb = tau_beta.iter().next().unwrap().1.total_number_of_paths_overflowing();
    if res_tb.1  {
        panic!("Overflow occurred while counting active S-boxes in Tau->Beta");
    }
    let sum_beta_paths: usize = res_tb.0;

    (sum_alpha_paths, sum_beta_paths)
}




