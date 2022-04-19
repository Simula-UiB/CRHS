

//! Module based on v3. Simply but, we pick one path and create the optimal alpha and beta path for
//! this path, independently of what alpha and beta paths are actually in the DAG. This ensures that
//! we get the most optimal weight for this particular path. As a drawback, if the alpha and/or beta
//! is not part of the alpha/beta paths for that particular hull, then the created alpha/beta may not
//! fit with all the paths we've found for the hull. If this approach or the extract alpha/beta
//! approach yields the best probability depends both on how "close" to optimal the alpha/beta path
//! extracted is/are, and on how many paths of the hull this approach forces us to skip. (And the
//! skipped paths weights).
//!
//! - Why paths are skipped, and a possible approach which will skip the fewest.
//! - Other issues with current implementation



use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;

use vob::Vob;
use vob::vob;

use crush::algebra::Matrix;
use crush::soc::bdd::Bdd as Shard;
use crush::soc::bdd::differential::{PPFactory, StyledProgressBar};

use crate::code_gen::SBoxHandler;
use crate::diff_solver::post_processing_v5::bt::bthandler_trait::BTHandler;
use crate::diff_solver::post_processing_v5::{Handlers, utils, AnalysisMode};
use crate::diff_solver::post_processing_v5::hull_calc::extract_alpha_beta::example_expanded_path;
use crate::diff_solver::post_processing_v5::bt::PROB_FACTOR;
use crate::diff_solver::post_processing_v5::hull_calc::{extract_all_paths_concurrently, ResultSectionBuilder, count_alpha_and_beta_paths_for_sess};
use crate::diff_solver::post_processing_v5::utils::path::Path;
use std::convert::TryInto;
use crate::diff_solver::post_processing_v5::logging::{Cache, AlphaBetaInnerPaths};


pub fn calculate_hull<B,S, P>(master: Arc<Shard>,
                              cache: &Cache,
                              lhss: &Matrix,
                              handlers: &Handlers<B,S>,
                              progress: &P,
                              result_builder2: &mut ResultSectionBuilder,
                              mode: &AnalysisMode
)
    where
        B: BTHandler,
        S: SBoxHandler,
        P: PPFactory,
{
    use AnalysisMode::*;

    let pb = progress.new_progress_bar(4);
    pb.set_message("Counting alpha/beta paths");
    let (sum_alpha_paths, sum_beta_paths) = count_alpha_and_beta_paths_for_sess(master.clone(), cache.master_md(), progress);

    let maybe_sum_inner_paths = cache.sum_sess_est_inner_paths();
    if maybe_sum_inner_paths.is_none() {
        panic!("Cannot calculate the sum of SESS Est paths when the SESS Est is not present");
    }
    let (sum_inner_paths, overflowed) = maybe_sum_inner_paths.unwrap();
    if overflowed {
        // TODO fixme log and handle
        panic!("Overflow occurred, the sum of the SESS Est's inner paths exceeds a usize!");
    }


    cache.record_abi_md(AlphaBetaInnerPaths::new(sum_alpha_paths, sum_beta_paths, sum_inner_paths));
    pb.println(&format!("sum inner paths: {}", sum_inner_paths));
    pb.inc(1);

    let block_size = handlers.bt_handler.sbox_layer_size();

    let (tx, inner_paths_rx) = extract_all_paths_concurrently::<P>(
        master.clone(),
        cache.master_md().alpha_lvl_depth,
        cache.master_md().beta_lvl_depth,
        200,
        progress.new_progress_bar(sum_inner_paths as u64),
    );

    pb.set_message("Creating alpha/beta path");
    let inner = inner_paths_rx.recv()
        .expect("Something went wrong with the inner paths coroutine. No paths were generated before it hung up");

    // Get alpha in and beta out.
    let (alpha_in, beta_out) = construct_alpha_beta(handlers,
                                                    &inner,
                                                    block_size,
                                                    cache.master_md().step,
                                                    lhss);

    result_builder2.alpha_path = Some(alpha_in.clone());
    result_builder2.beta_path = Some(beta_out.clone());
    pb.inc(1);

// Extracting an example path
    let example_path = example_expanded_path(
        inner.clone(),
        handlers,
        alpha_in.clone(),
        beta_out.clone(),
        lhss.clone(),
    );
    result_builder2.example_path = Some(example_path);

    // Return the "inner" path, so that we include it in the path probabilities!
    // Done in own thread, so not to risk a dead-lock. (This is a sync channel).
    thread::spawn(move || {let _ = tx.send(inner); });

    pb.set_message("Calculating probs");
    let path_influences = path_influences::<B, S, P>(inner_paths_rx,
                                                     cache.master_md().step,
                                                     alpha_in,
                                                     beta_out,
                                                     lhss,
                                                     handlers,
                                                     progress.new_progress_bar(sum_inner_paths as u64),
    );

    // Check if the hull calc was short-circuited due to reaching the path roof lim
    let path_influences = match path_influences {
        Ok(probs) => {
            // todo Set flag in result section, update printout of section
            probs
        },
        Err(probs) => {
            //  set flag in result section
            probs
            }
    };


    let path_influences: BTreeMap<usize, usize> = match mode{
        Linear => {
            // we need to double the infl's
            path_influences.into_iter()
                .map(|(infl, count)| (infl *2, count as usize))
                .collect()
        },
        Differential => {
            path_influences.into_iter()
                .map(|(p, c)| (p, c as usize) )
                .collect()
        },
    };


    result_builder2.probabilities_count = Some(path_influences.clone());
    // Paths marked with '0' weight are paths which had to be bypassed, meaning they were skipped
    result_builder2.paths_skipped = Some(match path_influences.get(&0) {
        None => 0,
        Some(c) => (*c).try_into().unwrap(),
    });
    pb.inc(1);

    // ================================== Debug ================================================
    // let pb = progress.new_progress_bar(5);
    // for (i, candidate) in path_probabilities.iter().take(5).enumerate() {
    //     pb.println(&format!("Prob nr {}: Prob is {}, count: {}", i, candidate.0, candidate.1));
    //     pb.inc(1);
    // }
    // pb.finish_and_clear();
    // ================================= Debug End =============================================

    // Calculate approximate hull
    let lowest = path_influences.iter().next().filter(|(k, _)| **k != 0);
    // First key may have been 0 => None. This is due to the path_prob bypass
    let lowest = match lowest {
        Some(val) => val,
        None => path_influences.iter().skip(1).next()
            .expect("We didn't find any non trivial weights!"),
    };
    //TODO fixme logg somewhere
    // println!("Lowest: {}", lowest.0);

    // Keys sorted by weight (ignore the 0 AKA the trivial), starting with k0 being the lowest:
    //   2^(-k0)*v0 + 2^(-k1)*v1 + 2^(-k2)*v2 ...
    // = 2^(-k0) * (v0 + 2^(k0-k1)*v1 + 2^(k0-k2)*v2 ...)
    // ...
    // 'return' k0 - x, where x = log2(v0 + 2^(k0-k1)*v1 + 2^(k0-k2)*v2 ...)

    pb.set_message("Calculating THE prob");
    let k0 = *lowest.0 as f64;
    let mut x = 0.0;
    // 'k' is stored as a usize in the bt_table, not as a f64. The "conversion" is achieved
    // using the prob_factor to move any decimals.
    let prob_factor = PROB_FACTOR as f64;
    for (k, v) in path_influences.iter().filter(|(k, _)| **k != 0) {

        x += *v as f64 / 2_f64.powf((*k as f64 - k0) / prob_factor);
    }

    result_builder2.hull_probability = Some( (k0 / prob_factor) - x.log2() );
    pb.inc(1);
    pb.finish_and_clear()
}

fn construct_alpha_beta<B: BTHandler, S: SBoxHandler>(handlers: &Handlers<B, S>,
                                                      inner: &Path,
                                                      block_size: usize,
                                                      step: NonZeroUsize,
                                                      lhss: &Matrix,)
                                                      -> (Path, Path) {
    let inner:Vob = inner.into();
    let alpha_in = construct_an_alpha(handlers.bt_handler(),
                                      &inner.iter().take(block_size).collect(),
                                      step.clone());

    let mut building = alpha_in.clone();
    building.append(&inner.into());
    let zero = vob![block_size; false];
    building.append(&zero.into());

    let building: Vob = building.into();
    #[allow(deprecated)]
    let expanded = expand_to_full_path(lhss, &building);

    // todo, no longer needs to be interleaved, can use path::expand to full instead.
    //In and out values are interleaved!
    let beta_interleaved: Vec<bool> = expanded.iter()
        .skip(expanded.len() - block_size*2)
        .collect();

    let mut beta_in = Vec::with_capacity(block_size);
    for (i, chunk) in beta_interleaved.chunks(step.get()).enumerate() {
        if i % 2 == 0 {
            beta_in.extend_from_slice(chunk);
        }
    }

    let beta_out = extract_a_beta(handlers.bt_handler(), &beta_in, step);

    (alpha_in.into(), beta_out.into())

}

/// Will calculate the "influence" each path has on the hull.
fn path_influences<B,S,P>(
    rx: Receiver<Path>,
    step: NonZeroUsize,
    alpha_in: Path,
    beta_out: Path,
    lhss: &Matrix,
    handlers: &Handlers<B, S>,
    pb: P::ProgressBar,
    // TODO make better Err
) -> Result<BTreeMap<usize, i32>, BTreeMap<usize, i32>>
    where
        B: BTHandler,
        S: SBoxHandler,
        P: PPFactory,
{
    pb.set_message("Calculating...");
    // Collect probability exponents here
    let mut all_influences = BTreeMap::new();
    let mut sum_paths = 0;

    //TODO make able to set by user
    let upper_lim = 2_usize.pow(26);

    for inner in rx {

        let mut rhs = alpha_in.clone();
        rhs.append(&inner);
        rhs.append(&beta_out.clone());


        // Expand to in values to the inner S-box layers:
        let extended = rhs;
        let extended = extended.expand_to_full_path(&lhss,
                                                    handlers.sb_handler(),
                                                    handlers.bt_handler()
                                                        .nr_of_rounds());

        // Calculate inner probabilities
        let influence = path_influence(&extended, handlers.bt_handler(), step); // FIXME document "wrong" prob if linear mode
        // NOTE: Prob is scaled by a constant factor in DT, which allows it to be stored as a usize
        // The factor chosen influences the rounding error.

        // FIXME if mode is linear,then double the prob value!

        let count = all_influences.entry(influence).or_insert(0);
        // Note That the path_probability bypass hack will return 0 as prob when bypassed
        // These 0's are NOT the trivial path...
        *count += 1;

        sum_paths += 1;
        pb.inc(1);

        // Warning: this means that we may skip good paths, make sure to document this fact
        if sum_paths >= upper_lim {
            return Err(all_influences);
        }

    }

    pb.finish_and_clear();
    Ok(all_influences)
}

/// For a given path, calculates the "influence" from the path on the hull. I.e. will lookup
/// the exact entries in the BT for the given in and out values.
fn path_influence<B: BTHandler> (extended: &Path,
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
    let s_boxes = utils::bools_to_lt8(&bools[..],
                                      step.get());

    // FIXME assumes in size == out size
    let nr_of_sboxes = bth.sbox_layer_size() / step.get();

    let mut sanity_check_bypass_invoked = false;
    // One chunk is the size of (non-id part of) in_state + out_state
    let res = s_boxes.chunks(2*nr_of_sboxes).enumerate()
        .flat_map(|(i, keys)| {
            let mut round = Vec::new();
            for j in 0..nr_of_sboxes {
                let bt = bth.bt(i, j);
                let entry = bt.get_entry(keys[j], keys[j + nr_of_sboxes])
                    .expect("Index out of bounds");


                // FIXME Sanity check bypassed b/c of restrictions introduced when calculating
                // alpha in and beta out. TODO Fix those restrictions, reintroduce sanity check
                // Sanity check
                // if entry == 0 {
                //     panic!(format!("We attempted to access a 0-entry in the BaseTable. Round: {}. Sbox pos {}. Row: {}, Col: {}",
                //                    i/ nr_of_sboxes,
                //                    i % nr_of_sboxes,
                //                    keys[0], keys[1],
                //     ))
                // };
                round.push((bt, entry));
            }
            round.into_iter()
        })
        // FIXME Sanity Check bypass: "continue" when entry == 0
        .inspect(|(_, entry)| if *entry == 0 {
            sanity_check_bypass_invoked = true;
        })
        .map(|(bt, val)| bt.prob_exponent_for_entry(val)
            .expect("Entry not present"))
        .sum();
    return if sanity_check_bypass_invoked {
        0
    } else {
        // FIXME if mode is linear,then double this value!
        res
    }

}

fn construct_an_alpha<H: BTHandler> (scheduler: &H, s_box_out: &Vec<bool>, step: NonZeroUsize) -> Path {
    let col_nr_s = utils::bools_to_lt8(&s_box_out, step.get());
    let mut out = Vec::with_capacity(col_nr_s.len());

    for (s_box_pos, col_nr) in col_nr_s.iter().enumerate() {
        let mut best = 0;
        let mut row = 1000;

        // Check the whole column for the best entry, where best is the highest entry
        let bt = scheduler.bt(0, s_box_pos);
        for i in 0..(bt.nr_of_columns()) {
            let entry = bt.get_entry(i as u8, *col_nr)
                .expect("Index out of bounds?");
            if entry > best {
                best = entry;
                row = i;
            }
        }

        if best == 0 { panic!("We didn't find a valid DDT entry!")}
        out.push(row);
    }

    let mut alpha = Vob::with_capacity(s_box_out.len());
    for row_i in out.iter() {
        let s = format!("{:0>8b}", row_i);
        for bit in s.chars().rev().take(step.get()) {
            alpha.push( bit.to_digit(2).unwrap() == 1);
        }
    }

    assert_eq!(s_box_out.len(), alpha.len());
    alpha.into()
}


fn extract_a_beta<H: BTHandler> (scheduler: &H, s_box_in: &[bool], step: NonZeroUsize) -> Vob {
    let row_nr_s = utils::bools_to_lt8(&s_box_in, step.get());
    let mut out = Vec::with_capacity(row_nr_s.len());

    for (s_box_pos, row_nr) in row_nr_s.iter().enumerate() {
        let mut best = 0;
        let mut col = 1000;

        let bt = scheduler.bt(scheduler.nr_of_rounds() -1, s_box_pos);
        let row = bt.row(*row_nr as usize)
            .expect("Something went wrong in the conversion");

        // Check the row for the best entry, where best is the highest entry
        for (i, entry) in row.iter().enumerate() {
            if *entry > best {
                best = *entry;
                col = i;
            }
        }
        if best == 0 { panic!("We didn't find a valid DDT entry!")}

        out.push(col);
    }

    let mut beta = Vob::with_capacity(s_box_in.len());
    for col in out.iter() {
        // FIXME check endianness!
        let s = format!("{:0>8b}", col);
        for bit in s.chars().rev().take(step.get()) {
            beta.push( bit.to_digit(2).unwrap() == 1);
        }
    }
    assert_eq!(s_box_in.len(), beta.len());
    beta
}

/// Perform a Mx = b op, where lhss is the M, and rhs = x. Returns b
#[deprecated]
fn expand_to_full_path(lhss: &Matrix, rhs: &Vob) -> Vob<usize> {
    let mut res = Vob::with_capacity(rhs.len()*2);

    for lhs in lhss.iter_rows() {
        let mut lhs = lhs.clone();
        lhs.and(rhs);
        res.push(lhs.iter_set_bits(..).count() % 2 == 1);
    }

    res
}