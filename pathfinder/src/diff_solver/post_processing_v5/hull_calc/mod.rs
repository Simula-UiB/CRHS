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
use crush::soc::Node;
pub use results::{ProcessedResult, DisplayResult, ProcessedResultSection};

use crate::code_gen::SBoxHandler;
use crate::diff_solver::post_processing_v5::bt::bthandler_trait::BTHandler;
use crate::diff_solver::post_processing_v5::{Handlers, SolvedSocMeta, AnalysisMode};
use crate::diff_solver::post_processing_v5::sess_handling::SessEstimate;
use crate::diff_solver::post_processing_v5::utils::path::Path;
use crate::diff_solver::post_processing_v5::logging::Cache;

pub mod construct_alpha_beta;
pub mod extract_alpha_beta;
mod results;

// Increments main_pb twice
pub fn calculate_hull<B,S, P>(master: Arc<Shard>,
                              cache: &Cache, lhss: Matrix,
                              handlers: &Handlers<B,S>,
                              main_pb: &P::ProgressBar,
                              progress: &P,
                              generic_res: ResultSectionBuilder,
                              mode: &AnalysisMode,
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
                                         &mut constructed_ab_result,
                                         mode
    );
    cache.record_processed_result_section(constructed_ab_result.clone().into());
    main_pb.inc(1);

    main_pb.set_message("Calc by extract");
    extract_alpha_beta::calculate_hull(master,
                                       cache,
                                       lhss,
                                       handlers,
                                       progress,
                                       &mut extract_result,
                                       mode,
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




