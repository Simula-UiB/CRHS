//! This approach will extract an alpha and a beta path from the available alpha/beta paths connected
//! to the chosen alpha and beta nodes. This ensures that all paths found connecting the alpha and beta
//! nodes are used in the calculation of the hull weight. However, as the alpha/beta path may not be
//! "optimal" it may or may not yield better results to use the `create_alpha_beta` approach.
//!
//! - Issues with this approach
//! - Some potential improvement to this approach
//!
//! Also need to address the potential improvements of the code in general. I.e. "code sharing" across
//! the approaches.


use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;

use vob::Vob;

use crush::algebra::Matrix;
use crush::soc::bdd::Bdd as Shard;

use crate::code_gen::SBoxHandler;
use crate::diff_solver::post_processing_v5::bt::bthandler_trait::BTHandler;
use crate::diff_solver::post_processing_v5::{bt, Handlers, hull_calc, SolvedSocMeta, utils};
use crate::diff_solver::post_processing_v5::hull_calc::{ ResultSectionBuilder};
use crate::diff_solver::post_processing_v5::hull_calc::extract_all_paths_concurrently;
use crate::diff_solver::post_processing_v5::utils::path::Path;
use crush::soc::bdd::differential::{PPFactory, StyledProgressBar};
use crate::diff_solver::post_processing_v5::logging::{Cache};

/// ************************* INVARIANTS ***********************************
/// Only the SESS start node is left on the Alpha level
/// Only the SESS end node is left on the Beta level
pub fn calculate_hull<B,S,P>(master: Arc<Shard>,
                             cache: &Cache,
                             lhss: Matrix,
                             handlers: &Handlers<B,S>,
                             progress: &P,
                             result_builder2: &mut ResultSectionBuilder,
)
    where
        B: BTHandler,
        S: SBoxHandler,
        P: PPFactory,
{

    let pb = progress.new_progress_bar(3);
    // Todo update other fn's to relax somewhat these invariants
    // ************************* INVARIANTS ***********************************
    // Only the SESS start node is left on the Alpha level
    // Only the SESS end node is left on the Beta level
    // This is assumed to be true for the remainder of functions:



    // Extract one alpha path and one beta path
    pb.set_message("Extracting alpha/beta path");
    // let sum_inner_paths = cache.sum_sess_est_inner_paths().expect("Best SESS Estimate should exist by now");
    let maybe_sum_inner_paths = cache.sum_sess_est_inner_paths();
    if maybe_sum_inner_paths.is_none() {
        panic!("Cannot calculate the sum of SESS Est paths when the SESS Est is not present");
    }
    let (sum_inner_paths, overflowed) = maybe_sum_inner_paths.unwrap();
    if overflowed {
        // TODO fixme log and handle
        panic!("Overflow occurred, the sum of the SESS Est's inner paths exceeds a usize!");
    }


    let (alpha_path, beta_path) = extract_alpha_beta_path(master.clone(), cache.master_md());
    result_builder2.alpha_path = Some(alpha_path.clone());
    result_builder2.beta_path = Some(beta_path.clone());
    pb.inc(1);

    // Extract all inner paths, running in a separate thread
    let (tx, inner_paths_rx) = extract_all_paths_concurrently::<P>(master.clone(),
                                                                   cache.master_md().alpha_lvl_depth,
                                                                   cache.master_md().beta_lvl_depth,
                                                                   200,
                                                                   progress.new_progress_bar(sum_inner_paths as u64),
    );

    // Extracting an example path
    let inner = inner_paths_rx.recv()
        .expect("Something went wrong with the inner paths coroutine. No paths were generated before it hung up");
    let example_path = example_expanded_path(
        inner.clone(),
        handlers,
        alpha_path.clone(),
        beta_path.clone(),
        lhss.clone(),
    );
    result_builder2.example_path = Some(example_path);

    // Return the "inner" path, so that we include it in the path probabilities!
    // Done in own thread, so not to risk a dead-lock. (This is a sync channel).
    thread::spawn(move || {let _ = tx.send(inner); });

    // For each inner path, combine with alpha and beta and make an expanded path,
    // then calculate their weights
    pb.set_message("Calculating probs");
    let path_probabilities = calculate_path_probabilities::<B, S, P>(cache.master_md().step,
                                                                     inner_paths_rx,
                                                                     handlers,
                                                                     alpha_path,
                                                                     beta_path,
                                                                     lhss,
                                                                     progress.new_progress_bar(sum_inner_paths as u64),
    );

    // Check if the hull calc was short-circuited due to reaching the path roof lim
    let path_probabilities = match path_probabilities {
        Ok(probs) => {
            // todo Set flag in result section, update printout of section
            probs
        },
        Err(probs) => {
            //  set flag in result section
            probs
        }
    };

    result_builder2.probabilities_count = Some(path_probabilities.clone());
    // Should always be '0'.
    result_builder2.paths_skipped = Some(match path_probabilities.get(&0) {
        None => 0,
        Some(c) => *c,
    });
    pb.inc(1);

    pb.set_message("Calculating THE prob");
    // Calculate and sum the contributions from each expanded path
        let lowest = path_probabilities.iter().next()
            .expect("No probabilities were found. Did we lose all the paths?");
        if *lowest.0 == 0 {
            panic!("We have found {} paths with 0 probability! And none should be the trivial path...", lowest.1);
        }

    // Keys sorted by weight (ignore the 0 AKA the trivial), starting with k0 being the lowest:
    //   2^(-k0)*v0 + 2^(-k1)*v1 + 2^(-k2)*v2 ...
    // = 2^(-k0) * (v0 + 2^(k0-k1)*v1 + 2^(k0-k2)*v2 ...)
    // ...
    // 'return' k0 - x, where x = log2(v0 + 2^(k0-k1)*v1 + 2^(k0-k2)*v2 ...)

    let k0 = *lowest.0 as f64;
    let mut x = 0.0;
    // 'k' is stored as a usize in the bt_table, not as a f64. The "conversion" is achieved
    // using the prob_factor to move any decimals.
    let prob_factor = bt::PROB_FACTOR as f64;
    // todo k should never be 0. Panic on 0?
    for (k, v) in path_probabilities.iter().filter(|(k, _)| **k != 0) {

        x += *v as f64 / 2_f64.powf((*k as f64 - k0) / prob_factor);
    }

    // Update with final result
    result_builder2.hull_probability = Some((k0 / prob_factor) - x.log2() );
    pb.inc(1);
    pb.finish_and_clear();
}

/// Extracts one alpha path and one beta path.
///
/// Invariants
/// Only the SESS start node is left on the Alpha level
/// Only the SESS end node is left on the Beta level
pub fn extract_alpha_beta_path(master: Arc<Shard>, master_md: &SolvedSocMeta) -> (Path, Path) {
    let alpha = hull_calc::extract_a_single_path(master.clone(), 0, master_md.alpha_lvl_depth);
    let sink_depth = master.get_sink_level_index();
    let beta =  hull_calc::extract_a_single_path(master, master_md.beta_lvl_depth, sink_depth);
    (alpha, beta)
}

/// Keys are the probability/weight, value is the number of paths with that probability/weight
pub fn calculate_path_probabilities<B, S, P> (
                                      step: NonZeroUsize,
                                      rx: Receiver<Path>,
                                      handlers: &Handlers<B, S>,
                                      alpha_path: Path,
                                      beta_path: Path,
                                      lhss: Matrix,
                                      pb: P::ProgressBar,
) -> Result<BTreeMap<usize, usize>, BTreeMap<usize, usize>>
    where
        B: BTHandler,
        S: SBoxHandler,
        P: PPFactory,
{
    pb.set_message("Calculating...");
    // Where all the probabilities are collected
    let mut all_probs = BTreeMap::new();
    let mut sum_paths = 0;
    //TODO make able to set by user
    let upper_lim = 2_usize.pow(26);

    let num_rounds = handlers.bt_handler().nr_of_rounds();
    let sbh = handlers.sb_handler();
    let bth = handlers.bt_handler();

    for inner in rx.iter() {

        let mut sierra_tau_path = alpha_path.clone();
        sierra_tau_path.append(&inner);
        sierra_tau_path.append(&beta_path.clone());

        let expanded = sierra_tau_path.expand_to_full_path(&lhss, sbh, num_rounds);


        // Calculate inner probabilities
        let prob = path_probability(&expanded, bth, step); // fixme Document that if mode is linear, then this prob is "wrong"
        // NOTE: Prob is scaled by a constant factor in DT, which allows it to be stored as a usize
        // The factor chosen influences the rounding error.

        // FIXME if mode is linear,then double this value!

        let count = all_probs.entry(prob).or_insert(0);
        *count += 1;
        sum_paths += 1;

        // Warning: this means that we may skip good paths, make sure to document this fact
        if sum_paths >= upper_lim {
            return Err(all_probs);
        }

        pb.inc(1);
    }

    pb.finish_and_clear();
    Ok(all_probs)
}

pub fn example_expanded_path<B,S>(
                              inner: Path,
                              handlers: &Handlers<B,S>,
                              alpha_path: Path,
                              beta_path: Path,
                              lhss: Matrix,
) -> Path
    where
        B: BTHandler,
        S: SBoxHandler,
{
    let num_rounds = handlers.bt_handler().nr_of_rounds();
    let sbh = handlers.sb_handler();

    let mut sierra_tau_path = alpha_path;
    sierra_tau_path.append(&inner);
    sierra_tau_path.append(&beta_path);

    let expanded = sierra_tau_path.expand_to_full_path(&lhss, sbh, num_rounds);
    expanded
}


//todo rename?
fn path_probability<B: BTHandler>(extended: &Path,
                        bth: &B,
                        step: NonZeroUsize,
) -> usize {
    // Maybe I could've done this using iter_set_bits() or iter_unset_bits directly
    // on the vob, and then using % step or similar to build the correct rows and col values
    // However, this is easier, and although the memory consumption is 8x as high, it should
    // still be within acceptable parameters.
    let bools: Vec<bool> = Vob::from(extended).iter().collect();

    // Convert into row and col values, we expect the input and output to the same s_box
    // to be adjacent.
    let s_boxes = utils::bools_to_lt8(&bools[..],step.get());

    // FIXME assumes in size == out size
    let nr_of_sboxes = bth.sbox_layer_size() / step.get();

    // One chunk is the size of (non-id part of) in_state + out_state
    let res = s_boxes.chunks(2*nr_of_sboxes).enumerate()
        .flat_map(|(i, keys)| {
            let mut round = Vec::new();
            for j in 0..nr_of_sboxes {
                let bt = bth.bt(i, j);
                let entry = bt.get_entry(keys[j], keys[j + nr_of_sboxes])
                    .expect("Index out of bounds");

                // Sanity check
                if entry == 0 {
                    panic!("We attempted to access a 0-entry in the BaseTable. Round: {}. Sbox pos {}. Row: {}, Col: {}",
                                   i/ nr_of_sboxes,
                                   i % nr_of_sboxes,
                                   keys[0], keys[1],
                    )
                };
                round.push((bt, entry));
            }
            round.into_iter()
        })
        .map(|(bt, val)| bt.prob_exponent_for_entry(val)
            .expect("Entry not present"))
        .sum();

        res

}
