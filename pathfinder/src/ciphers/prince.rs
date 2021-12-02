use std::collections::BTreeMap;
use std::convert::TryFrom;

use indicatif::ProgressBar;

use crush::soc::{Id, utils};
use crush::soc::bdd::differential::{PPFactory, StyledProgressBar};
use crush::soc::utils::*;

use crate::code_gen::gsf::GenericShard;
use crate::code_gen::SBoxHandler;
use crate::diff_solver::{OldSimpleSolver, SimpleSolver, SPFactory};
use crate::diff_solver::post_processing_v5::BaseTable;
use crate::diff_solver::post_processing_v5::BTHandler;

pub fn ddt_raw() -> Vec<Vec<usize>> {

    vec![  // 0, 1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13,  14,  15
        vec![16, 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0], // |  0
        vec![0,  4,  0,  0,  2,  0,  2,  0,  4,  2,  0,  2,  0,  0,  0,  0], // |  1
        vec![0,  2,  0,  4,  0,  0,  0,  2,  2,  0,  0,  0,  0,  4,  2,  0], // |  2
        vec![0,  0,  0,  0,  0,  2,  2,  0,  2,  2,  2,  2,  2,  0,  0,  2], // |  3
        vec![0,  2,  2,  4,  2,  2,  0,  0,  2,  0,  2,  0,  0,  0,  0,  0], // |  4
        vec![0,  0,  2,  2,  0,  2,  0,  2,  0,  2,  0,  2,  2,  2,  0,  0], // |  5
        vec![0,  0,  2,  2,  0,  2,  2,  0,  0,  2,  0,  2,  0,  0,  4,  0], // |  6
        vec![0,  0,  2,  0,  0,  0,  2,  0,  2,  0,  4,  0,  0,  2,  2,  2], // |  7
        vec![0,  0,  2,  0,  4,  2,  0,  0,  2,  2,  0,  2,  0,  2,  0,  0], // |  8
        vec![0,  0,  2,  2,  0,  0,  0,  0,  0,  2,  2,  0,  4,  2,  0,  2], // |  9
        vec![0,  0,  0,  2,  2,  4,  0,  4,  2,  0,  0,  0,  0,  0,  0,  2], // | 10
        vec![0,  2,  0,  0,  4,  0,  0,  2,  0,  0,  0,  2,  2,  0,  2,  2], // | 11
        vec![0,  4,  0,  0,  0,  2,  2,  0,  0,  0,  2,  2,  2,  0,  2,  0], // | 12
        vec![0,  2,  0,  0,  0,  0,  0,  2,  0,  4,  2,  0,  0,  2,  2,  2], // | 13
        vec![0,  0,  2,  0,  0,  0,  4,  2,  0,  0,  0,  2,  2,  2,  0,  2], // | 14
        vec![0,  0,  2,  0,  2,  0,  2,  2,  0,  0,  2,  0,  2,  0,  2,  2], // | 15
    ]
}

pub fn ddt_inverse_raw() -> Vec<Vec<usize>> {


    vec![     // 0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13,  14,  15
           vec![16,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0 ], // |  0
           vec![ 0,  4,  2,  0,  2,  0,  0,  0,  0,  0,  0,  2,  4,  2,  0,  0 ], // |  1
           vec![ 0,  0,  0,  0,  2,  2,  2,  2,  2,  2,  0,  0,  0,  0,  2,  2 ], // |  2
           vec![ 0,  0,  4,  0,  4,  2,  2,  0,  0,  2,  2,  0,  0,  0,  0,  0 ], // |  3
           vec![ 0,  2,  0,  0,  2,  0,  0,  0,  4,  0,  2,  4,  0,  0,  0,  2 ], // |  4
           vec![ 0,  0,  0,  2,  2,  2,  2,  0,  2,  0,  4,  0,  2,  0,  0,  0 ], // |  5
           vec![ 0,  2,  0,  2,  0,  0,  2,  2,  0,  0,  0,  0,  2,  0,  4,  2 ], // |  6
           vec![ 0,  0,  2,  0,  0,  2,  0,  0,  0,  0,  4,  2,  0,  2,  2,  2 ], // |  7
           vec![ 0,  4,  2,  2,  2,  0,  0,  2,  2,  0,  2,  0,  0,  0,  0,  0 ], // |  8
           vec![ 0,  2,  0,  2,  0,  2,  2,  0,  2,  2,  0,  0,  0,  4,  0,  0 ], // |  9
           vec![ 0,  0,  0,  2,  2,  0,  0,  4,  0,  2,  0,  0,  2,  2,  0,  2 ], // | 10
           vec![ 0,  2,  0,  2,  0,  2,  2,  0,  2,  0,  0,  2,  2,  0,  2,  0 ], // | 11
           vec![ 0,  0,  0,  2,  0,  2,  0,  0,  0,  4,  0,  2,  2,  0,  2,  2 ], // | 12
           vec![ 0,  0,  4,  0,  0,  2,  0,  2,  2,  2,  0,  0,  0,  2,  2,  0 ], // | 13
           vec![ 0,  0,  2,  0,  0,  0,  4,  2,  0,  0,  0,  2,  2,  2,  0,  2 ], // | 14
           vec![ 0,  0,  0,  2,  0,  0,  0,  2,  0,  2,  2,  2,  0,  2,  2,  2 ], // | 15
    ]
}


#[derive(Debug, Clone)]
pub struct PrinceBTScheduler {
    bt_forward: BaseTable,
    bt_inverse: BaseTable,
    cutoff: usize,
    nr_of_rounds: usize
}

impl PrinceBTScheduler {

    /// Prince1, nr of rounds: 4,
    /// Prince2, nr of rounds: 6,
    /// Prince3, nr of rounds: 8,
    /// Prince4, nr of rounds: 10,
    /// Prince5, nr of rounds: 12,
    pub fn new(nr_of_rounds: usize) -> Self {
        Self {
            bt_forward: BaseTable::try_from(self::ddt_raw()).unwrap(),
            bt_inverse: BaseTable::try_from(self::ddt_inverse_raw()).unwrap(),
            cutoff: nr_of_rounds / 2,
            nr_of_rounds,
        }
    }
}

impl BTHandler for PrinceBTScheduler {

    fn nr_of_rounds(&self) -> usize {
        self.nr_of_rounds
    }

    fn bt(&self, round: usize, _sbox_pos: usize) -> &BaseTable {
        if round >= self.cutoff {
            &self.bt_inverse
        } else {
            &self.bt_forward
        }
    }

    fn prob_exponents(&self, round: usize, _sbox_pos: usize) -> &BTreeMap<usize, usize> {
        if round >= self.cutoff {
            self.bt_inverse.prob_exponents()
        } else {
            self.bt_forward.prob_exponents()
        }
    }

    fn k(&self, round: usize, _sbox_pos: usize) -> f64 {
        if round >= self.cutoff {
            self.bt_inverse.k()
        } else {
            self.bt_forward.k()
        }
    }

    fn sbox_layer_size(&self) -> usize {
        64
    }

    fn prob_exponents_for_entry(&self, round: usize, _sbox_pos: usize, entry: usize) -> Option<usize> {
        if round >= self.cutoff {
            self.bt_inverse.prob_exponent_for_entry(entry)
        } else {
            self.bt_forward.prob_exponent_for_entry(entry)
        }
    }
}

pub struct SbMock {

}

impl SbMock {
    pub fn new() -> SbMock {
        SbMock{}
    }
}

impl SBoxHandler for SbMock {
    fn num_sboxes(&self, _round: usize) -> usize {
        16
    }

    fn sbox_size_in(&self, _round: usize, _pos: usize) -> usize {
        4
    }

    fn sbox_size_out(&self, _round: usize, _pos: usize) -> usize {
        4
    }

    fn bt_generic_shard(&self, _round: usize, _pos: usize) -> GenericShard {
        unimplemented!()
    }
}



#[derive(Debug, Clone)]
struct ProgressMock {
}

impl ProgressMock {
    fn new() -> ProgressMock {
        ProgressMock{}
    }

}

impl SPFactory for ProgressMock {
    type ProgressBar = StyledProgressMock;

    fn new_solve_progress(&self, len: u64) -> Self::ProgressBar {
        StyledProgressMock { pb: ProgressBar::new(len) }
    }
}

impl PPFactory for ProgressMock {
    type ProgressBar = StyledProgressMock;

    fn new_progress_bar(&self, len: u64) -> Self::ProgressBar {
        StyledProgressMock { pb: ProgressBar::new(len) }
    }
}

#[derive(Debug, Clone)]
struct StyledProgressMock {
    pb: ProgressBar,
}

impl StyledProgressBar for StyledProgressMock {
    fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    fn set_message(&self, msg: &str) {
        self.pb.set_message(msg);
    }

    fn finish_with_message(&self, msg: &str) {
        self.pb.finish_with_message(msg);
    }

    fn finish_and_clear(&self) {
        self.pb.finish_and_clear();
    }

    fn println(&self, msg: &str) {
        self.pb.println(msg);
    }
}

// ==================== Various Prince instances, from file ====================================

pub fn prince_1(soft_lim: usize) {
    let skip = 4;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "PRINCE_1.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);


    let mut rounds = vec![];
    let mut count = 0;
    for _ in 0..4 {
        let mut round = vec![];
        for _ in 0..16 {
            round.push(Id::new(count));
            count += 1;
        }
        rounds.push(round);
    }

    // let active_vars = Range{start: 64, end: 320 };
    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let _description = format!("PRINCE {}. Soft lim at: {}.", 1, soft_lim);
    let mut solver = SimpleSolver::new(soc, rounds, Id::new(0),
                                       var_mapping, 64, ProgressMock::new());
    solver.run(soft_lim);

    let path = &["out_results","Prince1_post_prune_lim20.bdd"].iter().collect();
    utils::print_system_to_file(solver.soc(), path);

    // let sys_spec = parse_system_spec_from_file(
    //     &["SoCs", "PRINCE_1.bdd"].iter().collect());
    // let soc = build_system_from_spec(sys_spec);
    // let pp = solver.finalize(Some(soc));
    // println!("{}",pp.summary());
    // pp.path_work_test();
}

pub fn prince_2(soft_lim: usize) {
    let skip = 4;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "PRINCE_2.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);


    let mut rounds = vec![];
    let mut count = 0;
    for _ in 0..6 {
        let mut round = vec![];
        for _ in 0..16 {
            round.push(Id::new(count));
            count += 1;
        }
        rounds.push(round);
    }

    // let active_vars = Range{start: 64, end: 320 };
    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let _description = format!("PRINCE {}. Soft lim at: {}.", 2, soft_lim);
    let mut solver = SimpleSolver::new(soc, rounds, Id::new(0),
                                       var_mapping, 64, ProgressMock::new());
    solver.run(soft_lim);

    let path = &["out_results","Prince2_post_prune.bdd"].iter().collect();
    utils::print_system_to_file(solver.soc(), path);



    // let sys_spec = parse_system_spec_from_file(
    //     &["SoCs", "PRINCE_2.bdd"].iter().collect());
    // let soc = build_system_from_spec(sys_spec);
    // let pp = solver.finalize(Some(soc));
    // println!("{}",pp.summary());
}

pub fn prince_3(soft_lim: usize) {
    let skip = 4;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "PRINCE_3.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);


    let mut rounds = vec![];
    let mut count = 0;
    for _ in 0..8 {
        let mut round = vec![];
        for _ in 0..16 {
            round.push(Id::new(count));
            count += 1;
        }
        rounds.push(round);
    }

    // let active_vars = Range{start: 64, end: 320 };
    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let _description = format!("PRINCE {}. Soft lim at: {}.", 3, soft_lim);
    let mut solver = SimpleSolver::new(soc, rounds,
                                       Id::new(0), var_mapping, 64,
                                       ProgressMock::new());
    solver.run(soft_lim);

    // let sys_spec = parse_system_spec_from_file(
    //     &["SoCs", "PRINCE_3.bdd"].iter().collect());
    // let soc = build_system_from_spec(sys_spec);
    // let pp = solver.finalize(Some(soc));
    // println!("{}",pp.summary());
}

pub fn prince_4(soft_lim: usize) {
    let skip = 4;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "PRINCE_4.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);


    let mut rounds = vec![];
    let mut count = 0;
    for _ in 0..10 {
        let mut round = vec![];
        for _ in 0..16 {
            round.push(Id::new(count));
            count += 1;
        }
        rounds.push(round);
    }

    // let active_vars = Range{start: 64, end: 320 };
    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let _description = format!("PRINCE {}. Soft lim at: {}.", 4, soft_lim);
    let mut solver = SimpleSolver::new(soc, rounds, Id::new(0),
                                       var_mapping, 64, ProgressMock::new());
    solver.run(soft_lim);

    // let sys_spec = parse_system_spec_from_file(
    //     &["SoCs", "PRINCE_4.bdd"].iter().collect());
    // let soc = build_system_from_spec(sys_spec);
    // let pp = solver.finalize(Some(soc));
    // println!("{}",pp.summary());
}


pub fn prince_5(soft_lim: usize) {
    let skip = 4;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "PRINCE_5.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);


    let mut rounds = vec![];
    let mut count = 0;
    for _ in 0..12 {
        let mut round = vec![];
        for _ in 0..16 {
            round.push(Id::new(count));
            count += 1;
        }
        rounds.push(round);
    }

    // let active_vars = Range{start: 64, end: 320 };
    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let _description = format!("PRINCE {}. Soft lim at: {}.", 5, soft_lim);
    let mut solver = SimpleSolver::new(soc, rounds, Id::new(0),
                                       var_mapping, 64,
                                       ProgressMock::new());
    solver.run(soft_lim);

    let path = &["out_results","Prince5_post_prune_lim16.bdd"].iter().collect();
    utils::print_system_to_file(solver.soc(), path);

    // let sys_spec = parse_system_spec_from_file(
    //     &["SoCs", "PRINCE_5.bdd"].iter().collect());
    // let soc = build_system_from_spec(sys_spec);
    // let pp = solver.finalize(Some(soc));
    // println!("{}",pp.summary());
}

pub fn prince_5_old(soft_lim: usize) {
    let skip = 4;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "PRINCE_5.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);


    let mut rounds = vec![];
    let mut count = 0;
    for _ in 0..12 {
        let mut round = vec![];
        for _ in 0..16 {
            round.push(Id::new(count));
            count += 1;
        }
        rounds.push(round);
    }

    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let mut solver = OldSimpleSolver::new(soc, rounds, Id::new(0),
                                          var_mapping, ProgressMock::new());
    solver.run(soft_lim);
}