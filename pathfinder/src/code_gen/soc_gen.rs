use crate::code_gen::{LLHandler, SBoxHandler};
use vob::Vob;
use crush::soc::system::System;
use crush::soc::Id;


// Observations:
// There are essentially a few things that we need in order to generate a SoC:
//      1) Block size, assuming all blocks are of the same size.
//          If not, we need to know the various block sizes, and when they apply.
//      2) Which S-box is applied when.
//          If incomplete s-box layer, how many are applied that round.
//          How many in and out bits to the S-box.
//      3) Which linear layer is applied when/at what round.
//
//  1) Tells us the size of our initial in block. This block is used as in bits to the first
// non-linear layer (when complete), and as output of of the identity part of the non-linear layer
// (when incomplete layer).
//  2) Tells us which S-box we shall base the RHS of the shard upon, as well as how many
// in bits/linear combinations and out bits/linear combinations the LHS has (= levels in RHS (excluding
// sink)). Collective knowledge from here and block size allows us to construct the correct out-block
// (of the non-linear layer == in block to linear layer).
// 3) Tells us which linear transformation we shall apply to the out-block, in order to create the
// next in-block.



// Transitions from one round to another is defined to be at any application of a non-linear layer,
// partial or complete alike. That means that round 0 is from the initial input to the cipher,
// through any linear transformation, and then as input to the first non-linear layer. The shards
// for round 0 will be the shards made form that input, and from the fresh variables which are the
// output of the non-linear part of the non-linear layer.
//
// The terms 'in' and 'out' will in general refer to the input and output of a non-linear layer.
pub fn make_soc<L, S>(llb: &L, sh: &S, nr_rounds: usize) -> (System, Vec<Vec<Id>>)
    where
        L: LLHandler,
        S: SBoxHandler,
{
    let nvar = count_nvar(llb, sh, nr_rounds);
    let mut shards = Vec::new();

    // Initial in-block:
    let init_block_size = llb.block_size(0);
    let mut initial = Vec::with_capacity(init_block_size);
    for i in 0..init_block_size {
        let mut lhs = Vob::from_elem(nvar, false);
        lhs.set(i, true);
        initial.push(lhs);
    }
    // Apply round 0 linear layer,
    // 'inn' is input block to the non-linear layer.
    // let mut inn = llb.apply_linear_layer(0, initial);
    let mut inn = initial;

    // Used by the simple_solver to ensure its invariants
    let mut rounds: Vec<Vec<Id>> = Vec::new();

    let mut next_var_id = init_block_size;
    let mut next_shard_id = 0;
    for r in 0..nr_rounds {
        rounds.push(Vec::with_capacity(sh.num_sboxes(r)));
        // Block size of out round
        let block_size = llb.block_size(r+1);
        debug_assert_eq!(block_size, inn.len(), "At round: {}", r); // todo is this correct?

        // Output block to non-linear layer
        let mut out = Vec::with_capacity(block_size);
        // Setup for consuming inn: First for shard gen, then rest (if any) is moved into out
        let mut inn_iter = inn.into_iter();

        // Make shards:
        for s in 0..sh.num_sboxes(r) {

            // Make LHSs for the out bits of next shard to be created
            let mut lhs_o = Vec::new();
            for _ in 0..sh.sbox_size_out(r, s) {
                let mut lhs_out =  Vob::from_elem(nvar, false);
                lhs_out.set(next_var_id, true);
                next_var_id += 1;

                lhs_o.push(lhs_out.clone());
                // Duplicate for the linear layer transformation
                out.push(lhs_out);
            }

            // Create specific shard
            let shard = sh.bt_generic_shard(r, s)
                .into_specific(&mut inn_iter,&mut lhs_o.into_iter(),
                               Id::new(next_shard_id));
            shards.push(shard);
            rounds[r].push(Id::new(next_shard_id));
            next_shard_id += 1;
        }

        // Ensure that the out state is ready to go through the linear layer
        out.extend(inn_iter);

        // Apply linear layer
        // Skip linear layer iff we're at the last round
        if r == nr_rounds - 1 {
            break;
        }
        inn = llb.apply_linear_layer(r+1, out);
    }

    // Apply last round linear layer? This is only needed if the post-State is returned somehow,
    // as the post-State isn't connected to any Shards. (Post-State is state after last application
    // of non-linear layer).

    // Make and return SoC. Now also includes the overview of what Shards are at what rounds
    (System::from_elem(shards).unwrap(), rounds)

}

fn count_nvar(llc: &dyn LLHandler, sc: &dyn SBoxHandler, nr_rounds: usize) -> usize {
    // Account for initial in block variables
    let mut n_vars = llc.block_size(0);

    for r in 0..nr_rounds {
        // Add the count of out-bits for each S-box
        for s in 0..sc.num_sboxes(r) {
            n_vars += sc.sbox_size_out(r, s);
        }
    }
    n_vars
}