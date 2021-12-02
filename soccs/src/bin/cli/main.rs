use std::borrow::Borrow;
use std::thread;
use std::time::Duration;

use structopt::StructOpt;

use crush::soc::bdd::differential::StyledProgressBar;
use dl_options::DlOptions;
use pathfinder::diff_solver::post_processing_v5::DisplayResult;
use soccs::dl::{DLmode, OutFiles, RawSoc, Setup, SolvedSoC, StopAfter};
use soccs::dl::builders::cg::{BtHandler, CgBuilder, SbHandler};
use soccs::dl::cg_original::cipher::{Cipher, name_to_cipher, prince};
use soccs::dl::progress::{MyStyledSpinner, Progress};
use crate::batches::*;

mod dl_options;
mod batches;

fn main() {

    match DlOptions::from_args() {
        DlOptions::Diff {
            cipher,
            soft_lim,
            soft_lim_exponent,
            num_rounds,
            out_parent_folder,
            in_parent_folder,
            silent_mode,
        } => {

            // Prince is the only cipher that actually behaves differently after the reflective round.
            // In the original CryptaGraph implementation, this was bypassed. Unfortunately for us,
            // this bypass means that the original CG implementation of Prince breaks CG's own Cipher
            // trait: It always returns the "forward" S-box, even when it should return the inverse
            // S-box.
            // In order to fix this, any Prince instance needs to know how many rounds it is. I.e.
            // when we've passed the reflective round and should get the inverse S-box instead. In order
            // to do this in a backwards compatible way, we decided to add a num_rounds: Option<usize>
            // field in Prince. This means that we need to set the num_rounds manually after creation,
            // which in turns means that we must create the Prince instance manually. (Trait Cipher
            // does not have a "set_num_rounds" fn.)
            // (We could have used Any, but that would mean adding more changes to the original
            // Prince impl).

            let cipher: Box<dyn Cipher + Send> =
                if cipher == "prince".to_string() {

                    let mut prince = prince::Prince::new();
                    prince.set_num_rounds(num_rounds);
                    Box::new(prince)
                } else {
                    match name_to_cipher(cipher.as_ref()) {
                        Some(c) => c,
                        None => {
                            println!("Cipher not supported. Check --help for supported ciphers.");
                            return;
                        }
                    }
                };

            let soft_lim = unwrap_soft_lim(soft_lim, soft_lim_exponent);
            let out_files = OutFiles::new(out_parent_folder, &cipher.name(), num_rounds, &DLmode::Differential, soft_lim);

            let setup = Setup::new(
                cipher.name(),
                cipher.structure(),
                num_rounds,
                soft_lim,
                DLmode::Differential,
                StopAfter::Process,
                out_files,
                in_parent_folder,
                silent_mode,
            );

            run(setup, cipher);
        },

        DlOptions::Lin {
            cipher,
            soft_lim,
            soft_lim_exponent,
            num_rounds,
            out_parent_folder,
            in_parent_folder,
            silent_mode,
        } => {

            // Prince is the only cipher that actually behaves differently after the reflective round.
            // In the original CryptaGraph implementation, this was bypassed. Unfortunately for us,
            // this bypass means that the original CG implementation of Prince breaks CG's own Cipher
            // trait: It always returns the "forward" S-box, even when it should return the inverse
            // S-box.
            // In order to fix this, any Prince instance needs to know how many rounds it is. I.e.
            // when we've passed the reflective round and should get the inverse S-box instead. In order
            // to do this in a backwards compatible way, we decided to add a num_rounds: Option<usize>
            // field in Prince. This means that we need to set the num_rounds manually after creation,
            // which in turns means that we must create the Prince instance manually. (Trait Cipher
            // does not have a "set_num_rounds" fn.)
            // (We could have used Any, but that would mean adding more changes to the original
            // Prince impl).

            let cipher: Box<dyn Cipher + Send> =
                if cipher == "prince".to_string() {

                    let mut prince = prince::Prince::new();
                    prince.set_num_rounds(num_rounds);
                    Box::new(prince)
                } else {
                    match name_to_cipher(cipher.as_ref()) {
                        Some(c) => c,
                        None => {
                            println!("Cipher not supported. Check --help for supported ciphers.");
                            return;
                        }
                    }
                };

            let soft_lim = unwrap_soft_lim(soft_lim, soft_lim_exponent);
            let out_files = OutFiles::new(out_parent_folder,
                                          &cipher.name(),
                                          num_rounds,
                                          &DLmode::Linear,
                                          soft_lim);

            let setup = Setup::new(
                cipher.name(),
                cipher.structure(),
                num_rounds,
                soft_lim,
                DLmode::Linear,
                StopAfter::Process,
                out_files,
                in_parent_folder,
                silent_mode,
            );

            run(setup, cipher);

        },

        DlOptions::CG {
            soft_lim_exponent,
            out_parent_folder,
            in_parent_folder,
            batch,
            linear,
            differential,
        } => {

            if (linear | differential) == false {
                println!("Neither liner nor differential is set to be run. At least one of them must be set for any analysis to be done.");
                return;
            }

            let mut setups_to_run = vec![];

            if differential {
                match batch {
                    0 => setups_to_run.append(
                        &mut as_from_paper_diff_batch_0(soft_lim_exponent.unwrap())),
                    1 => setups_to_run.append(
                        &mut as_from_paper_diff_batch_1(soft_lim_exponent.unwrap())),
                    2 => setups_to_run.append(
                        &mut as_from_paper_diff_batch_2(soft_lim_exponent.unwrap())),
                    3 => setups_to_run.append(
                        &mut as_from_paper_diff_batch_3(soft_lim_exponent.unwrap())),
                    4 => setups_to_run.append(
                        &mut as_from_paper_diff_batch_4(soft_lim_exponent.unwrap())),
                    _ => panic!("Batch not found!"),
                };
            }

            if linear {
                match batch {
                    0 => setups_to_run.append(
                        &mut as_from_paper_lin_batch_0(soft_lim_exponent.unwrap())),
                    1 => setups_to_run.append(
                        &mut as_from_paper_lin_batch_1(soft_lim_exponent.unwrap())),
                    2 => setups_to_run.append(
                        &mut as_from_paper_lin_batch_2(soft_lim_exponent.unwrap())),
                    3 => setups_to_run.append(
                        &mut as_from_paper_lin_batch_3(soft_lim_exponent.unwrap())),
                    4 => setups_to_run.append(
                        &mut as_from_paper_lin_batch_4(soft_lim_exponent.unwrap())),
                    _ => panic!("Batch not found!"),
                };
            }

            // Prince is the only cipher that actually behaves differently after the reflective round.
            // In the original CryptaGraph implementation, this was bypassed. Unfortunately for us,
            // this bypass means that the original CG implementation of Prince breaks CG's own Cipher
            // trait: It always returns the "forward" S-box, even when it should return the inverse
            // S-box.
            // In order to fix this, any Prince instance needs to know how many rounds it is. I.e.
            // when we've passed the reflective round and should get the inverse S-box instead. In order
            // to do this in a backwards compatible way, we decided to add a num_rounds: Option<usize>
            // field in Prince. This means that we need to set the num_rounds manually after creation,
            // which in turns means that we must create the Prince instance manually. (Trait Cipher
            // does not have a "set_num_rounds" fn.)
            // (We could have used Any, but that would mean adding more changes to the original
            // Prince impl).

            for settings in setups_to_run {

                let cipher: Box<dyn Cipher + Send> =
                    if settings.cipher == "prince".to_string() {
                        let mut prince = prince::Prince::new();
                        prince.set_num_rounds(settings.num_rounds);
                        Box::new(prince)
                    } else {
                        match name_to_cipher(&settings.cipher) {
                            Some(c) => c,
                            None => {
                                println!("Cipher not supported. Check --help for supported ciphers.");
                                return;
                            }
                        }
                    };

                // This allows for cipher dependent adjustments. More specifically, it allows AES
                // and Khazad to set a lower soft lim, as they have a higher S-box bit count.
                let soft_lim = unwrap_soft_lim(None, Some(settings.soft_lim_e));

                let mut out_parent_folder = out_parent_folder.clone();
                out_parent_folder.push(settings.out_parent_folder.clone());

                let out_files = OutFiles::new(out_parent_folder,
                                              &cipher.name(),
                                              settings.num_rounds,
                                              &settings.mode.clone(),
                                              soft_lim);

                let setup = Setup::new(
                    cipher.name(),
                    cipher.structure(),
                    settings.num_rounds,
                    soft_lim,
                    settings.mode.clone(),
                    StopAfter::Process,
                    out_files,
                    in_parent_folder.clone(),
                    true,
                );

                run(setup, cipher);
            }

        }


    }

}

fn unwrap_soft_lim(soft_lim: Option<usize>, soft_lim_exponent: Option<usize>) -> usize {
    if let Some(sl) = soft_lim {
        sl
    } else if let Some(exp) = soft_lim_exponent {
        2_usize.pow(exp as u32)
    } else {
        panic!("No soft limit detected, unable to proceed.");
    }
}

// ===============================================================================================
// ===============================================================================================

/// Cipher cant implement clone, so I need a duplicate from the beginning...
fn run(setup: Setup, cipher: Box<dyn Cipher + Send>) {
    let progress_arena = Progress::new();
    let main_pb = init_main_pb(&progress_arena, &setup, &cipher.name());


    let solved_soc =
        if setup.in_parent_folder().is_none()
        {
            main_pb.set_message("Building SoC");
            drive_progress(progress_arena.clone());

            let build_progress = progress_arena.new_spinner();
            // build raw
            build_progress.set_message("Building SoC from cipher specs.");
            let raw_soc = from_beginning(&setup, cipher);
            build_progress.finish_and_clear();
            main_pb.inc(1);

            // then solve
            main_pb.set_message(&format!("Solving: {}", setup.cipher_name()));
            raw_soc.solve_soc(&setup, progress_arena.clone())

        } else
        {
            main_pb.set_message("Loading SoC from file");
            drive_progress(progress_arena.clone());

            from_solved_soc(&setup,progress_arena.new_spinner(), cipher.as_ref())
        };

    main_pb.inc(1);


    match setup.stop_after() {
        StopAfter::Solve => {
            main_pb.finish_with_message("SoC solved, we're done!");
            //  Allow main pb to be shut down, avoids mix-ups in the final printout
            thread::sleep(Duration::from_secs(1));
            return;
        },
        _ => {},
    }

    main_pb.set_message("Analysing the Solved SoC");
    let result = solved_soc.analyse(progress_arena.clone())
        // TODO update error handling as error handling improves
        .expect("Something went wrong");

    main_pb.inc(1);
    main_pb.finish_with_message("All done!");
    //  Allow main pb to be shut down, avoids mixups in the final printout
    thread::sleep(Duration::from_secs(1));
    // Print result

    if !setup.silent_mode() {
        // FIXME made into comments as quickfix
        println!("{}", DisplayResult::AsSummary(&result));
        // let buff = result.1.print().unwrap();
        // println!("{}", buff);
    }
}

///
fn init_main_pb(progress_arena: &Progress,
                setup: &Setup,
                cipher_name: &str,
) -> MyStyledSpinner
{
    let main_pb = progress_arena.new_main_spinner();
    main_pb.println(
        &format!("Received Cipher: {}, num rounds: {}, soft limit: {}, mode: {}",
                 cipher_name, setup.num_rounds(), setup.soft_lim(), setup.dl_mode(),
        ));
    main_pb.enable_steady_tick(1000);

    main_pb
}

/// Drives the progress bar. Not calling this may result in deadlocks. See progress.rs for more.
fn drive_progress(progress_arena: Progress) {
    let _ = thread::spawn(move || {
        progress_arena.join();
    });
}


/// Builds the SoC "from the beginning". I.e. from the cipher spec, as an unsolved cipher,
/// rather than from a file (where the SoC may, in theory, be in any state: Solved, partially
/// solved or unsolved).
#[inline]
fn from_beginning(setup: &Setup, cipher: Box<dyn Cipher>)
                  -> RawSoc<BtHandler, SbHandler>
{
    CgBuilder::from_cipher(setup, cipher.borrow())
}


/// Builds the SoC from file. Even though a SoC read in from file can, in theory, be in any
/// state (solved, partially solved or unsolved), we will assume that the SoC we read in is
/// a Solved SoC.
/// Feeding any other state to this fn is undefined behaviour.
#[inline]
fn from_solved_soc(setup: &Setup, progress_spinner: MyStyledSpinner, cipher: &dyn Cipher)
                   -> SolvedSoC<BtHandler, SbHandler, Progress>
{
    CgBuilder::from_parent_folder(
        setup,
        cipher,
        progress_spinner,
        setup.in_parent_folder()
            .expect("Why call from parent folder when no parent folder is given?")
            .clone(),
    )
}