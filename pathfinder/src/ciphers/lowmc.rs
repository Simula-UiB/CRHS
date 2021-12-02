use crush::soc::Id;
use crush::soc::utils::*;

use crate::diff_solver::SimpleSolver;

pub fn lowmc_9_1_3(soft_lim: usize) {
    let skip = 3;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_3.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let mut rounds = vec![];
    for i in 0..=soc.iter_bdds().count()-1 {
        rounds.push(vec![Id::new(i)]);
    }

    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let description = format!("LowMc_{}_{}_{}. Soft lim at: {}.\nNon standard Linear layer.",
                              9, 1, 3, soft_lim);
    let mut solver = SimpleSolver::new(soc, &description,
                                       rounds, Id::new(0), var_mapping, 9);
    solver.run(soft_lim);


    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_3.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let pp = solver.finalize(Some(soc));
    println!("{}",pp.summary());
}

pub fn lowmc_9_1_6(soft_lim: usize)  {
    let skip = 3;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_6.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let mut rounds = vec![];
    for i in 0..=soc.iter_bdds().count()-1 {
        rounds.push(vec![Id::new(i)]);
    }

    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let description = format!("LowMc_{}_{}_{}. Soft lim at: {}.\nNon standard Linear layer.",
                              9, 1, 6, soft_lim);
    let mut solver = SimpleSolver::new(soc, &description,
                                       rounds, Id::new(0), var_mapping, 9);
    solver.run(soft_lim);

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_6.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let pp = solver.finalize(Some(soc));
    pp.path_work_test();
    println!("{}",pp.summary());
}

pub fn lowmc_9_1_6_old(soft_lim: usize) {
    use crate::diff_solver::OldSimpleSolver;
    let skip = 3;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_6.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let mut rounds = vec![];
    for i in 0..=soc.iter_bdds().count()-1 {
        rounds.push(vec![Id::new(i)]);
    }

    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let mut solver = OldSimpleSolver::new(soc, rounds, Id::new(0), var_mapping);
    solver.run(soft_lim);
}


pub fn lowmc_9_1_8(soft_lim: usize) {
    let skip = 3;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_8.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let mut rounds = vec![];
    for i in 0..=soc.iter_bdds().count()-1 {
        rounds.push(vec![Id::new(i)]);
    }

    let description = format!("LowMc_{}_{}_{}. Soft lim at: {}.\nNon standard Linear layer.",
                              9, 1, 8, soft_lim);
    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let mut solver = SimpleSolver::new(soc, &description,
                                       rounds, Id::new(0), var_mapping, 9);
    solver.run(soft_lim);

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_8.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let pp = solver.finalize(Some(soc));
    println!("{}",pp.summary());
    pp.path_work_test();
}

pub fn lowmc_9_1_35(soft_lim: usize) {
    let skip = 3;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_35.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let mut rounds = vec![];
    for i in 0..=soc.iter_bdds().count()-1 {
        rounds.push(vec![Id::new(i)]);
    }

    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let description = format!("LowMc_{}_{}_{}. Soft lim at: {}.\nNon standard Linear layer.",
                              35, 1, 6, soft_lim);
    let mut solver = SimpleSolver::new(soc, &description,
                                       rounds, Id::new(0), var_mapping, 9);
    solver.run(soft_lim);

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_35.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let pp = solver.finalize(Some(soc));
    println!("{}",pp.summary());
}

pub fn lowmc_9_1_70(soft_lim: usize) {
    let skip = 3;

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_70.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let mut rounds = vec![];
    for i in 0..=soc.iter_bdds().count()-1 {
        rounds.push(vec![Id::new(i)]);
    }

    let var_mapping = soc.iter_bdds()
        .map(|(id, shard)| {
            let to_keep = shard.borrow().get_lhs().iter().skip(skip)
                .cloned()
                .collect();
            (id.clone(), to_keep)
        }).collect();

    let description = format!("LowMc_{}_{}_{}. Soft lim at: {}.\nNon standard Linear layer.",
                              9, 1, 70, soft_lim);
    let mut solver = SimpleSolver::new(soc, &description,
                                       rounds, Id::new(0), var_mapping, 9);
    solver.run(soft_lim);

    let sys_spec = parse_system_spec_from_file(
        &["SoCs", "LowMC9_1_70.bdd"].iter().collect());
    let soc = build_system_from_spec(sys_spec);
    let pp = solver.finalize(Some(soc));
    println!("{}",pp.summary());
}