use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::Arc;

use crush::algebra::Matrix;
use crush::soc::bdd::Bdd as Shard;
use crush::soc::bdd::differential::{Depth, PPFactory, StyledProgressBar};
use crush::soc::bdd::differential::wd::{Node2NodeDistribution, WDPresence, WDLevel, EndNodeDist};
use crush::soc::bdd::differential::wd::NWDistribution;
use crush::soc::Id;

use crate::code_gen::SBoxHandler;
// todo fix reference to old mod
use crate::diff_solver::post_processing_v5::logging::{Cache, LogType};
use crate::diff_solver::post_processing_v5::sess_handling::*;
use crate::diff_solver::post_processing_v5::hull_calc::{ResultSectionBuilder, BuildMode};

pub use hull_calc::{ProcessedResult, DisplayResult};

mod sess_handling;
mod logging;
mod utils;
mod bt;
mod hull_calc;
// mod hull_calc_v2;

pub use bt::{BaseTable, bthandler_trait::BTHandler};
pub use sess_handling::{SessEstimate, InnerWeight};
pub use logging::TraceLogger;
use std::fmt::Debug;
use std::sync::mpsc::Sender;

pub struct SolvedSocMeta {
    active_area: Range<usize>,
    step: NonZeroUsize,
    alpha_lvl_depth: Depth,
    beta_lvl_depth: Depth,
}

pub struct Handlers<B: BTHandler, S: SBoxHandler> {
    bt_handler: B,
    sb_handler: S,
}

pub enum AnalysisMode{
    Differential,
    Linear,
}


pub fn start_post_processing<B, S, P> (
    mut master: Shard,
    master_meta: SolvedSocMeta,
    lhss: Matrix,
    handlers: Handlers<B, S>,
    progress: P,
    _cipher_name: String,
    tx: Sender<LogType>,
    mode: AnalysisMode,

) -> ProcessedResult
    where
        B: BTHandler + Debug,
        S: SBoxHandler,
        P: PPFactory,
{
    let main_pb = progress.new_progress_bar(5);
    main_pb.set_message("Starting PP!");
    //quickfix, out destination to be given as param, not created here

    // let mut meta_data_out_file = current_dir().expect("Couldn't access current working directory...");
    // meta_data_out_file.push(format!("{}_{}",cipher_name, "trace"));
    // meta_data_out_file.set_extension("txt");
    // main_pb.println(&format!("{}", meta_data_out_file.display()));

    // let settings = Settings {
    //     meta_data_out_file,
    // };


    // quickfix, logger to be given as param, not made here
    // let tx = TraceLogger::init_and_run(settings.meta_data_out_file.clone());

    let mut cache = Cache::new(tx, master_meta);

    main_pb.set_message("Estimating best SESS");
    // Find the best SESS estimate, based on the average weight an active S-box will contribute.
    let best_estimate = estimate_best_sess(&master,
                                                 &handlers,
                                                 &progress,
                                                 &mut cache,
    );
    cache.register_sess_estimate(best_estimate.clone());
    main_pb.inc(1);



    //todo register in cache!
    main_pb.set_message("Deleting superfluous nodes");
    delete_non_sess_estimate_nodes(&mut master, &cache.master_md(), &best_estimate);
    main_pb.inc(1);


    main_pb.set_message("Calculating hull");
    let master = Arc::new(master);

    // Make the "result" template, containing the metadata the final results will contain.
    let generic_result_builder = ResultSectionBuilder::new(
        BuildMode::Template,
        best_estimate,
        // todo document assumption that all layers are the same size
        handlers.bt_handler.sbox_layer_size(),
        handlers.bt_handler.nr_of_rounds(),
    );

    let res = hull_calc::calculate_hull(
        master,
        &mut cache,
        lhss,
        &handlers,
        &main_pb,
        &progress,
        generic_result_builder,
        &mode,
    );


    main_pb.finish_with_message("PP done!");
    res
}

// =============================================================================================

impl SolvedSocMeta {
    pub fn new(active_area: Range<usize>,
               step: NonZeroUsize,
               alpha_lvl_depth: Depth,
               beta_lvl_depth: Depth,) -> Self {
        SolvedSocMeta {
            active_area,
            step,
            alpha_lvl_depth,
            beta_lvl_depth
        }
    }
}

impl<B,S > Handlers<B, S>
    where
    B: BTHandler,
    S: SBoxHandler,
{
    pub fn new(bt_handler: B,
               sb_handler: S,) -> Self {

        Self {
            bt_handler,
            sb_handler,
        }
    }

    pub fn bt_handler(&self) -> &B {
        &self.bt_handler
    }

    pub fn sb_handler(&self) -> &S { &self.sb_handler }
}


// =============================================================================================


fn estimate_best_sess<B,S,P>(master: &Shard,
                                   handlers: &Handlers<B, S>,
                                   progress: &P,
                                   cache: &mut Cache,
) -> SessEstimate
    where
        B: BTHandler,
        S: SBoxHandler,
        P: PPFactory,
{
    // ============================= Setup for SESS Con Search ====================================
    let pb = progress.new_progress_bar(9);
    pb.set_message("Making Sierra->Alpha");
    cache.make_and_analyse_sierra_alpha(master, progress);
    pb.inc(1);

    // Find alpha MESS Con candidates
    pb.set_message("Making Tau->Alpha");
    let alpha_level = cache.make_and_analyse_tau_alpha(master);
    let alpha_level_len = alpha_level.len();
    pb.inc(1);

    pb.set_message("Finding candidates");
    let (alpha_level_nt_lew, alpha_candidates) = alpha_candidates(alpha_level);
    cache.analyse_alpha_candidates(alpha_level_nt_lew, &alpha_candidates);
    pb.inc(1);

    pb.set_message("Analysing MESSCon's");
    cache.analyse_mess_cons(master, alpha_level_len, progress);
    pb.inc(1);

    // Find best SESS Con candidates: Setup
    pb.set_message("Making Tau->Beta");
    let beta_level = Arc::new(cache.make_and_analyse_tau_beta(master));
    pb.inc(1);
    pb.set_message("Making Alpha->Beta");
    let alpha_beta_dists = make_alpha_beta_level(&alpha_candidates, cache, master, progress);
    pb.inc(1);

    // ============================= Estimate best SESS ====================================

    // Find best SESS Con candidates: Search phase
    // Done sequentially for each nt_lew in alpha_candidates. If need be, it should be fairly
    // straightforward to transform it into a concurrent setup.

    // Max number of connections we wish to receive back.
    let max_connections = 20;
    let mut candidate_sess_es = Vec::with_capacity(3*max_connections);

    pb.set_message("Making SESS Cons");
    for alpha_candidate_vec in alpha_candidates.into_iter() {
        if alpha_candidate_vec.is_empty() {
            pb.inc(1);
            continue;
        }
        let candidates_nt_lew = alpha_candidate_vec.get(0).unwrap()
            .1.lowest_existing_non_trivial_weight().unwrap();

        pb.set_message(&format!("Making SESS Cons: {}", candidates_nt_lew));
        let candidates = estimate_best_sess_connections (
            alpha_candidate_vec,
            beta_level.clone(),
            alpha_beta_dists.clone(),
            alpha_level_nt_lew,
            candidates_nt_lew,
            max_connections,
            progress,
            // FIXME assumes all S-boxes will yield the same k, which we have to assume for now.
            handlers.bt_handler.k(0,0),
        );
        candidate_sess_es.extend_from_slice(&candidates);
        pb.inc(1)
    }
    if candidate_sess_es.is_empty() {
        panic!("No non-trivial SESS was found!");
    }

    pb.finish_and_clear();
    candidate_sess_es.sort_unstable_by(|a, b| {
        // OBS! We're comparing f64's here, something I am a bit queasy to do, but since
        // I'm not looking for exact matches anyways, I think we're fine.
        b.estimate().partial_cmp(&a.estimate()).unwrap()
    });


    // Get candidate
    let mut candidate = candidate_sess_es.swap_remove(0);

    // Get distribution linking alpha and beta nodes for the candidate.
    let sess_dist = alpha_beta_dists
        .get(&candidate.end())
        .expect("End node is missing!")
        .other_node(&candidate.start())
        .expect("Start node is missing!");
    //Sess dist is the WDCount dist for *all* paths connecting SESS start and SESS end.
    candidate.set_hull_distribution( sess_dist.clone());

    candidate
}


/// Deleting all nodes which are not relevant for the hull.
/// Deleting these other nodes will leave us with only the inner paths which are relevant for our
/// differential/hull calculation, saving us space (we no longer need an WDArena for the inner
/// levels, which could take more space than master itself!), and also saving us computation (no
/// longer need to check every node in every cohort to see which path takes us to the End Node.
/// Now they all do).
fn delete_non_sess_estimate_nodes(master: &mut Shard, master_md: &SolvedSocMeta, sess_estimate: &SessEstimate) {
    // Deleting from beta level
    let beta_level = master.level(master_md.beta_lvl_depth)
        .expect("Beta level is missing");

    let mut to_delete = beta_level.get_nodes().clone();
    let target_node = to_delete.remove(&sess_estimate.end());
    // Sanity check
    if target_node.is_none() {
        panic!("We failed for some reason to remove the End Node from the set of nodes to be deleted");
    }
    master.delete_all_marked_nodes_from_level(to_delete.keys().collect(),
                                              master_md.beta_lvl_depth);

    // Deleting from alpha level
    let alpha_level =  master.level(master_md.alpha_lvl_depth)
        .expect("Alpha level is missing");

    let mut to_delete = alpha_level.get_nodes().clone();
    let target_node = to_delete.remove(&sess_estimate.start());
    // Sanity check
    if target_node.is_none() {
        panic!("We failed for some reason to remove the Start Node from the set of nodes to be deleted");
    }
    master.delete_all_marked_nodes_from_level(to_delete.keys().collect(),
                                              master_md.alpha_lvl_depth);
}

/// The construction of the Alpha->Beta WDLevel is the most memory intensive part of the analysis,
/// and may easily far bypass the memory consumption of the Master shard, even pre-pruning!
/// Special care is therefore needed, and this fn intends to provide just that care.
///
/// My current main theory for what the issue may be, is that if we have 'many' candidates, and
/// also a very wide widest level (or maybe even before!), even the total number of Dists drives the
/// memory consumption too high. My plan to fix this is to see if we cannot batch handle a smaller
/// amount of candidates at a time, before folding them together into a final WDLevel. This *will*
/// increase time consumption (maybe unless I manage to parallelize it w/o returning to the original
/// problem), but should allow us to keep the memory consumption within reason.
///
/// For now, I will not do this, as finding the right batch size will require some work, and I'm out
/// of time....
fn make_alpha_beta_level(alpha_candidates: &Vec<Vec<(Id, WDPresence)>>,
                         cache: &mut Cache,
                         master: &Shard,
                         progress: &impl PPFactory)
-> Arc<WDLevel<EndNodeDist>>
{
    let (_widest, _s_widest) = cache.record_master_layout(master);

    let targets: Vec<Id> = alpha_candidates.iter()
        .flat_map(|vec| vec.iter())
        .map(|(id, _)| id.clone())
        .collect();

    let alpha_beta_dists = Arc::new(cache.make_and_analyse_alpha_beta(master, targets, progress));

    alpha_beta_dists
}