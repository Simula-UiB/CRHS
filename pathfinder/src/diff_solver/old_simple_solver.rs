use std::cell::{Ref, RefMut};
use std::collections::HashMap;
use std::ops::Range;

use vob::Vob;

use crush::algebra::{self, Matrix};
use crush::soc::bdd::Bdd;
use crush::soc::Id;
use crush::soc::system::System;

use crate::diff_solver::meta::{AbsorbRec, JoinRec, PreAbsorbRec};
use crate::diff_solver::SPFactory;

use super::meta::{Librarian, Ops};
use super::meta::CoreOps::*;
use super::meta::Ops::*;

pub type Depth = usize;

pub struct OldSimpleSolver<F>
    where
        F: SPFactory,
{
    soc: System,
    master_id: Id,
    var_mapping: HashMap<Id, Vec<Vob>>,
    /// All shards, sorted by what round they were created in.
    rounds: Vec<Vec<Id>>,
    /// Id's of all `Shard`s which have been joined into `Master`, including `Master`'s own original `Id`.
    joined_w_master: Vec<Id>,
    step: usize,
    librarian: Librarian<F>,
}

impl<F: SPFactory> OldSimpleSolver<F> {
    pub fn new(soc: System,
               rounds: Vec<Vec<Id>>,
               master_id: Id,
               var_mapping: HashMap<Id, Vec<Vob>>,
               progress: F,
    )
               -> Self
    {
        let joined_w_master = vec![master_id.clone()];
        let step = var_mapping.iter().next().unwrap().1.len();
        // Invariant chek on 'step'
        for lhss in var_mapping.values() {
            if lhss.len() != step {panic!(
                "Currently only supports the same number of LHS's to be associated with an S-box, but got two different values: {}, {}", step, lhss.len());
            }
        }

        let librarian = Librarian::new(soc.get_size(), progress);
        let me = Self {
            soc,
            master_id,
            var_mapping,
            rounds,
            joined_w_master,
            step,
            librarian,
        };
        me
    }



    pub fn run(&mut self, soft_lim: usize) {
        use console::style;

        if self.rounds.len() == 0 { panic!("We cannot check a primitive with no rounds!")}

        // Quickfix, to bypass some borrow rule mistake I must have made
        let roundss = self.rounds.clone();

        // First round, we expect that it may take a few joins before we can absorb dependencies
        for id in roundss[0].iter() {
            if id == &self.master_id { continue }
            self.join_op(*id);

            self.resolve_any_deps();
            self.check_prune(soft_lim);
        }

        if roundss.len() > 1 {

            // We expect to be able to absorb all linear dependencies immediately after joining, as
            // the variables they depend upon should already exist in 'master'
            for round in roundss.iter().skip(1).take(self.rounds.len() - 2) {
                for id in round.iter() {
                    self.join_op(*id);

                    self.resolve_any_deps();
                    self.check_prune(soft_lim);
                }
            }
        }

        if roundss.len()  == 1 { return }
        // Last round, depending on how last round is generated, we may get some LHS's which are
        // neither a linear dependency, nor one that is to be included in the active area.
        // The expensive way to deal with those are to move them all the way to the top, the cheaper
        // way is to move them all the way to the bottom. (And then drop them?)
        for id in roundss.last().unwrap().iter() {
            // FIXME currently handled as any round
            self.join_op(*id);

            self.resolve_any_deps();
            self.check_prune(soft_lim);
        }

        // quickfix'es
        // self.librarian.print();
        let arena = self.master().identify_trails_and_weights(
            self.active_area(), self.step);
        self.master().map_highest_lsb(&arena);

        println!("\n{:=^150}", "");
        println!("{:=^150}", style(" DONE ").green());
        println!("{:=^150}", "");

        println!("Test path finder");
        // for (i, (lhs, edge)) in self.master()
        //     .extract_an_lsb_path(&self.active_area(), self.step)
        //     .iter().enumerate() {
        //
        //     println!("{: >4}: {} => {}", i, &format!("{:?}",lhs), *edge as u8);
        // }
        println!("{}", self.master().extract_a_sol(&self.active_area(), self.step));
        println!("Sol as hex:\n{}" ,self.master().stringify_sol_as_hex(&self.active_area(), self.step));
    }

    pub fn soc(&self) -> &System {
        &self.soc
    }
}


impl<F> OldSimpleSolver<F>
    where
        F: SPFactory,
{

    /// Since joining ended up having some bookkeeping associated with it, it got its own fn.
    /// As it is right now, this may slow things down a little. (Calculating lin deps may be slow).
    fn join_op(&mut self, bottom: Id) {
        self.soc.join_bdds(self.master_id, bottom).expect("Join failed");
        self.joined_w_master.push(bottom);

        let lhs = self.master().get_lhs();
        let dependencies = algebra::extract_linear_dependencies(matrix![lhs]);

        self.librarian.record(Join(
            JoinRec::new(self.master_id, bottom, self.soc.get_size(), dependencies.row_size())));


        self.pre_prune();
    }

    /// Absorbs any linear dependencies present in `Master`.
    ///
    /// Note that 'soft lim' should be set with enough distance from 'hard lim' such that it won't
    /// be necessary to prune before all dependencies have been absorbed. Excluding the first round,
    /// and assuming a 'standard' setup of the SoC, then we have the following rule of thumb:
    /// - One linear absorption can in worst case double the complexity of `Master`, therefore
    /// > hard lim - soft lim >= s^a
    ///
    /// where 'a' is the number of absorptions to be done. Those are typically equal to the number of
    /// input bit to an S-box.
    fn resolve_any_deps(&mut self) {
        let lhs = self.master().get_lhs();
        let mut dependencies = algebra::extract_linear_dependencies(matrix![lhs]);

        if dependencies.is_empty() {
            return;
        }

        // Absorb all dependencies
        while !dependencies.is_empty() {
            let dep = self.next_to_resolve(dependencies);
            self.resolve_dep(dep);
            // Update dependency matrix
            let lhs = self.soc.get_bdd(self.master_id).unwrap().borrow().get_lhs();
            dependencies = algebra::extract_linear_dependencies(matrix![lhs]);
        }
    }

    /// Ensures that the `prune invariants` are upheld
    fn pre_prune(&mut self) {
        self.librarian.record(Text(format!("\nPre-pruning:\n")));

        // ** First, resolve any dependencies, that is almost always the best course of action and
        // we're trying to keep it simple.
        self.resolve_any_deps();

        // ** Second, ensure that the active area only contains the relevant levels **

        // Depth of all levels outside active area but should have been inside
        let mut move_in = Vec::new();
        // Depth of all levels inside active area but should have been outside
        let mut move_out = Vec::new();

        let active_area = self.active_area();

        // LHS's supposed to be inside active area
        let expected_inside:Vec<&Vob> = self.var_mapping_for_master().iter()
            .flat_map(|(_, lhss)| lhss.iter())
            // .cloned()
            .collect();

        let all_lhss = self.master().get_lhs();
        // Split based on whether the levels (depths) are inside or outside active range
        let (outside, inside) = all_lhss.split_at(active_area.start);

        // Check outside
        for (depth, lhs) in outside.iter().enumerate() {
            if expected_inside.contains(&lhs) {
                move_in.push(depth);
            }
        }
        // Here, in theory, we could skip checking active area for "inactive" levels if
        // move_in.len() == 0, as move_in and move_out should be equal. However, until I can prove
        // that theory for myself and other, I'll take the performance penalty of checking both.

        // Checking active area, aka inside
        for (depth, lhs) in inside.iter().enumerate() {
            let offset = outside.len();
            if !expected_inside.contains(&lhs) {
                move_out.push(depth + offset );
            }
        }

        // Handle any levels which are not in the right region.
        if move_out.len() > 0 || move_in.len() > 0 {
            // This strategy depends on the theoretical fact that the number of levels needed to be
            // moved into the other region should be the same.
            debug_assert_eq!(move_out.len(), move_in.len(), "Unexpected difference in the number of levels to be moved");

            // Important that they are indeed sorted, or the swapping will not be "stable"
            // FIXME is the sorting going in the correct order?
            move_in.sort();
            move_out.sort();
            let mut next_pos = active_area.start;
            for depth in move_out.iter() {
                self.swap(*depth, next_pos);
                next_pos += 1;
            }
            next_pos -= 1;
            for depth in move_in.iter().rev() {
                self.swap(*depth, next_pos);
                next_pos -= 1;
            }
            debug_assert_eq!(next_pos +1, active_area.start); // To remove, should only fail if I made a coding mistake
        }

        // =================================================================================
        // ** Third, ensure that all levels supposed to be adjacent indeed are adjacent. **
        //FIXME fix any consistency breaches, instead of panicking!

        #[cold]
        #[inline(never)]
        fn err_message(msg: &str, s_box: &Id, m_depth: usize, r_start: usize, r_end: usize) -> String {
            let range = format!("Active range in 'Master': {}..{}", r_start, r_end);
            let s_box = format!("S-box: {}.\nError at depth {} in 'Master'", s_box, m_depth);
            format!("{}\n{}\n{}", msg, range, s_box)
        }



        // Mapping from the lhs's to their depth
        let lhs_depth: HashMap<Vob, Depth> = self.master().get_lhs()
            .iter().enumerate()
            .map(|(i, vob)| (vob.clone(), i))
            .collect();

        // Consistency check
        // Concept: All LHS's from the same S-box should be adjacent. Therefore, for each relevant S-box,
        // get the 'Depth's for the relevant LHS's and sort the vector. Then, starting with the
        // second lowest Depth, check that the next Depth in the vec is equal to previous Depth + 1
        // in Master.
        for (sbox, lhss) in self.var_mapping_for_master().iter() {
            // Getting relevant Depths, may be in arbitrary order
            let mut depths: Vec<Depth> = lhss.iter()
                .map(|lhs| *lhs_depth.get(&lhs)
                    .expect("We seem to have lost a LHS we wanted to keep...")
                )
                .collect();

            depths.sort();
            // Check for adjacency
            let mut prev = *depths.get(0).expect("Should always be at least one LHS");
            for depth in depths.iter().skip(1) {
                prev += 1;
                if *depth != prev {
                    let msg ="Level not adjacent to the other levels from the same S-box!";
                    panic!("{}", err_message(msg, sbox, *depth,
                                       active_area.start, active_area.end));
                }
                if !active_area.contains(depth) {

                    let msg = "Depth not within active range!";
                    panic!("{}", err_message(msg, sbox, *depth,
                                       active_area.start, active_area.end));
                }
            }
        }
    }

    /// Will execute prune if soft_lim is exceeded
    fn check_prune(&mut self, soft_lim: usize) {
        if self.soc.get_size() > soft_lim {
            // self.pre_prune();
            // self.librarian.record(Text(format!("Pruning")));

            let active_area = self.active_area();

            let mut helper = Librarian::<F>::record_prune_helper();

            let _ = self.master_mut()
                .complexity_based_wide_prune_v2(soft_lim,
                                                active_area,
                                                self.step,
                                                &mut helper,
                );
            self.librarian.record(Ops::Prune(helper.get_rec().unwrap())); // FIXME

        }
    }

    /// Returns a Range indicating which levels of the `Master` shard which needs to
    /// abide with the `prune invariants`.
    /// (The prune invariants can be found in its respective mod).
    fn active_area(&self) -> Range<usize> {
        let end = self.master().get_sink_level_index();
        let start = end - (self.step * self.joined_w_master.len());

        Range{start, end}
    }

    /// Return the Id's of the S-boxes which are now part of `Master`, and their respective
    /// associated variables/LHS's. This is a map from Id to a Vec of VoB's, where each VoB
    /// represents an LHS associated with that Id.
    fn var_mapping_for_master(&self) -> HashMap<&Id, &Vec<Vob>> {
        self.var_mapping.iter()
            .filter(|(id, _lhs)| {
                self.joined_w_master.contains(id)
            })
            .collect()
    }

    /// self.master contains the `*Id*` of `Master`, whereas this method will
    /// return a reference to the `Master` *shard*.
    /// Bypasses the `.unwrap().borrow()`...
    #[inline]
    fn master(&self) -> Ref<Bdd> {
        self.soc.get_bdd(self.master_id).unwrap().borrow()
    }

    /// self.master contains the `*Id*` of `Master`, whereas this method will
    /// return a mutable reference to the `Master` *shard*.
    /// Bypasses the `.unwrap().borrow()`...
    #[inline]
    fn master_mut(&self) -> RefMut<Bdd> {
        self.soc.get_bdd(self.master_id).unwrap().borrow_mut()
    }

    /// Swap level at `from` in `Master` such that it ends up at `to`, i.e. level which was at
    /// depth `from` will be at depth `to` when the function returns.
    ///
    /// - If `from` is less then `to`, (`from` is above `to` in the shard), then all levels between
    /// `from` and `to`, inclusive `to`, will be shifted one depth upwards (previous depth -1).
    /// - If `from` is greater than `to`, all the levels in between (inclusive `to`) will end up at
    /// previous depth + 1. (One depth lower than before).
    /// - If `from` == `to`, then nothing happens.
    fn swap(&mut self, from: Depth, to: Depth) {
        let mut current = from;
        // Shift downwards
        if current < to {
            while current < to {
                self.soc.swap(self.master_id, current, current + 1)
                    .expect(&format!("Current was at: {}. Master depth: {}. Swap failed",
                                     current,
                                     self.master().get_levels_size()));
                current += 1;
            }
            // Else, shift upwards
        } else if current > to {
            while current > to {
                self.soc.swap(self.master_id, current - 1, current)
                    .expect(&format!("Current was at: {}. Master depth: {}. Swap failed",
                                     current,
                                     self.master().get_levels_size()));
                current -= 1;
            }
        }
    }

    /// Absorb the given "linear dependency".
    fn resolve_dep(&mut self, dependency: Vob) {


        let (mut base, rest, pre_abs) = self.pre_absorb(dependency);

        let mut recording = AbsorbRec::new();
        recording.set_pre_abs_rec(pre_abs);

        for next in rest.iter() {
            self.swap(base, next + 1);
            recording.record(Swap(base, next+1));

            self.soc.add(self.master_id, *next, next + 1).expect("Add failed.");
            recording.record(Add(*next, next+1),);

            base = next + 1;
        }

        self.soc.absorb(self.master_id, base, false).expect("Level extraction failed.");
        recording.record(Extract(base));
        recording.set_complexity(self.soc.get_size());
        self.librarian.record(Absorb(recording))
    }

    /// Conforms the given linear dependency to the expectations of 'resolve_dep()'.
    ///
    /// Orders the positioning of `base` in relation to the rest of the involved `levels` in such
    /// a way that `base` is either the bottom most involved level, or directly below the top most
    /// involved level. Also, it ensures that the order of the vec `involved` is the correct
    /// relative to where `base` ends up.
    ///
    /// Returns (base, rest), where 'rest' is a vec of all involved levels/depths except 'base'
    fn pre_absorb(&mut self, dependency: Vob) -> (Depth, Vec<Depth>, PreAbsorbRec) {

        let mut involved: Vec<Depth> = dependency.iter_set_bits(..).collect();
        // It is my understanding that the MSB in the dependency matrix corresponds to depth 0
        // in our master shard. It is also my understanding that Vob.iter_set_bits() starts indexing
        // from the MSB. Thus, index 0 should correspond to depth 0.

        debug_assert!(!involved.is_empty(), "Somehow we have a linear dependency, but with none involved levels!?!");

        // If the linear dependency consists of only two involved levels, then the two levels
        // must be the same linear combination. It is safe to resolve this dependency, even if the
        // linear combination is part of the var_mapping.
        if involved.len() == 2 {
            let base = involved.remove(1);
            let inv_clone = involved.clone();
            return (base, involved, PreAbsorbRec::new(base.clone(), inv_clone));
            // return (involved.remove(1), involved)
        }


        // Vec of LHS's to keep, that is, which should not be base
        let lhss:Vec<&Vob> = self.var_mapping_for_master().iter()
            .flat_map(|(_, lhss)| lhss.iter())
            .collect();

        // Checking bottom
        let last = self.master().get_lhs_level(*involved.last().unwrap() );
        if !lhss.contains(&&last) {
            let base = involved.pop().unwrap();
            involved = involved.into_iter().rev().collect();

            let inv_clone = involved.clone();
            return (base, involved, PreAbsorbRec::new(base.clone(), inv_clone));
        }

        // else, find a LHS which can be absorbed, and either leave it at top(if it already is top),
        // move it to the bottom, or move it to adjacent level below top. Whichever is shortest.

        for (i, depth) in involved.iter().take(involved.len() - 2 ).enumerate() { // -2, bottom is checked
            let lhs = self.master().get_lhs_level(*depth);
            if lhss.contains(&&lhs) {
                continue;
            }

            // Now we know that we are free to absorb this lhs.
            let top = involved.first().unwrap().clone();
            if depth == &top {
                let base = involved.remove(0);
                let invo_clone = involved.clone();
                return (base, involved, PreAbsorbRec::new(base.clone(), invo_clone));
            }

            // Else, 'lhs' is not 'top' nor 'bottom' => swap to nearest edge
            let bottom = involved.last().unwrap().clone();
            let mut swaps = Vec::new();

            // bottom is closest
            return if bottom - depth < depth - top {
                self.swap(*depth, bottom);
                swaps.push(Swap(*depth, bottom));

                //Updating 'involved''s depths, including any passed through as part of the swap
                involved.remove(i);
                for a_depth in involved.iter_mut().skip(i) {
                    *a_depth -= 1;
                }
                // Reverse the order, so that resolve_dep() starts with the 'bottom', instead of 'top'.
                involved = involved.into_iter().rev().collect();

                let mut pre_abs = PreAbsorbRec::new(bottom.clone(), involved.clone());
                pre_abs.record_swaps(swaps);

                (bottom, involved, pre_abs)

            } else {
                // top is closest, or equal distance to top and bottom
                // Moving to adjacent to top, but we don't actually touch top
                self.swap(*depth, top + 1);
                swaps.push(Swap(*depth, top+1));
                // Swapping does not change 'involved'...
                involved.remove(i);
                for a_depth in involved.iter_mut().take(i).skip(1) {
                    *a_depth += 1;
                }
                let base = top + 1;

                let mut pre_abs = PreAbsorbRec::new(base.clone(), involved.clone());
                pre_abs.record_swaps(swaps);

                (base, involved, pre_abs)
            }
        }
        panic!("For some reason, all the LHS's in this linear dependency are marked as ones we cannot absorb!");
    }

    /// Returns the linear dependency with the shortest distance, in terms of edges traversed, between
    /// top involved level and bottom involved level. If several linear dependencies share the shortest
    /// distance, then the first encountered is returned.
    /// The linear dependency is returned as a tuple, where the first is the `depth` of `base`, while
    /// the second is a vector with the `depth` of the remaining involved levels.
    ///
    /// NOTE: This associated method assumes that all involved levels are part of the same shard.
    ///
    /// Expects `matrix` to be the linear dependency matrix.
    /// Returns None if no linear dependencies are present.
    fn next_to_resolve(&self, matrix: Matrix) -> Vob<usize> {

        let mut best_dep: Depth = Depth::max_value();
        let mut shortest_distance = usize::max_value();
        for (i, row) in matrix.iter_rows().enumerate() {
            let mut set_bits = row.iter_set_bits(..);
            let lsb = set_bits.next().expect("We expect a linear dependency to include at least two levels.");
            let msb = set_bits.last().expect("We expect a linear dependency to include at least two levels.");

            if msb - lsb < shortest_distance {
                shortest_distance = msb - lsb;
                best_dep = i;
            }
        }



        matrix.get_row(best_dep).unwrap().clone()

    }

}

// #[cfg(test)]
// mod test {
//     use super::*;
//     #[test]
//     fn test() {
//         let st = r#"9;1;[("1+2",[(1;1,1)])]"#.to_owned();
//         let te = bdd!(st);
//         let shard = bdd!(9;1;[("1+2",[(1;1,1)])]);
//         println!("Test shard: {:#?}", shard);
//
//         panic!("Forced panic!");
//     }
// }
// // Need:
// // - master as Shard
// // - master as Id
// // - a var mapping, mapping which levels are to be kept adjacent (for counting), and which shard
// // they can be found in
// // - step, but this can be gained from the var map
// // - soc, well, it's the SoC, pretty self-explanatory
// // - rounds, a vec of rounds, where each round is a vec with the id of the shards originating from
// // that round
// // - soft and hard lim, may be calculated
//
// // Invariants
// // - step is always the same
// // - active area is always ending in sink (where sink is excluded), and is step * (sboxes in master) levels
// // - levels in active area are adjacent according to the var mapping. Order is otherwise irrelevant
// // - active area start is always at "the beginning" of an "adjacent section"
//
// // Fn's potentially suitable for incorporating into a Diff_solver trait
// // To be learned from this, as SimpleSolver essentially is a Diff_solver
//
// // Fn's potentially suitable for incorporation into a Diff_cipher trait
// //


