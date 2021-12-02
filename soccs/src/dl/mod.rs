use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Error as FmtError, Debug};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Result as IoResult};
use std::io::Write;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::PathBuf;

use vob::Vob;

use crush::algebra::Matrix;
use crush::soc::bdd::Bdd as Shard;
use crush::soc::bdd::differential::PPFactory;
use crush::soc::Id;
use crush::soc::system::System;
use pathfinder::code_gen::{LLHandler, SBoxHandler};
use pathfinder::diff_solver::{Librarian, SimpleSolver, SolverResultOk, SPFactory};
// use pathfinder::diff_solver::post_processing_v3::{PostPFactory, PostProc, ProcessedResult as ProcessedResultV3};
use pathfinder::diff_solver::post_processing_v5::{BTHandler, TraceLogger, AnalysisMode};
use pathfinder::diff_solver::post_processing_v5::{Handlers, SolvedSocMeta, start_post_processing, DisplayResult, ProcessedResult};

use crate::dl::progress::Progress;
use crate::dl::cg_original::cipher::{CipherStructure};

pub mod builders;
pub mod cg_original;
pub mod dl_options;
pub mod progress;

#[derive(Debug, Clone)]
pub struct Setup {
    cipher_name: String,
    cipher_structure: CipherStructure,
    num_rounds: usize,
    soft_lim: usize,
    mode: DLmode,
    stop_after: StopAfter,
    out_files: OutFiles,
    in_parent_folder: Option<PathBuf>,
    silent_mode: bool,
}

impl Setup {
    pub fn new(
        cipher_name: String,
        cipher_structure: CipherStructure,
        num_rounds: usize,
        soft_lim: usize,
        mode: DLmode,
        stop_after: StopAfter,
        out_files: OutFiles,
        in_parent_folder: Option<PathBuf>,
        silent_mode: bool)
        -> Self {
        Self {
            cipher_name,
            cipher_structure,
            num_rounds,
            soft_lim,
            mode,
            stop_after,
            out_files,
            in_parent_folder,
            silent_mode
        }
    }

    #[inline]
    pub fn cipher_name(&self) -> String {
        self.cipher_name.to_string()
    }

    #[inline]
    pub fn num_rounds(&self) -> usize {
        self.num_rounds
    }

    #[inline]
    pub fn soft_lim(&self) -> usize {
        self.soft_lim
    }

    #[inline]
    pub fn dl_mode(&self) -> DLmode {
        self.mode.clone()
    }

    #[inline]
    pub fn stop_after(&self) -> StopAfter {
        self.stop_after.clone()
    }

    #[inline]
    pub fn out_files(&self) -> &OutFiles {
        &self.out_files
    }

    #[inline]
    pub fn in_parent_folder(&self) -> Option<&PathBuf> {
        self.in_parent_folder.as_ref()
    }

    #[inline]
    pub fn silent_mode(&self) -> bool {
        self.silent_mode
    }
}

/// What stages should be completed before we are done?
#[derive(Debug, Clone)]
pub enum StopAfter {
    /// Build and solve the SoC, but then quit.
    /// Useful if all you want for know is the solved SoC, for later analysis.
    Solve,
    /// Build, Solve and Post Process. I.e. the full package.
    Process,
}

/// Various target files for output data
#[derive(Debug, Clone)]
pub struct OutFiles {
    out_parent_folder: PathBuf,
    /// Output file for the .bdd file
    bdd_file: PathBuf,
    /// Output file for any logg from the Solving Process
    pruning_logg: PathBuf,
    /// Output destination for the Logg trace of PostProcessing.
    /// The Logg trace has provided much needed debug info, as it prints "live"
    logg_trace: PathBuf,
    /// Output file for the final results logg of PostProcessing.
    pp_logg: PathBuf,
}

impl OutFiles {
    pub fn new(out_parent_folder: PathBuf,
               cipher_name: &str,
               num_rounds: usize,
               mode: &DLmode,
               soft_lim: usize)
               -> Self
    {
        let core_filename = Self::derive_core_filename(cipher_name, num_rounds, mode.clone(), soft_lim);

        let mut bdd_file = out_parent_folder.clone();
        bdd_file.push(core_filename.clone());
        bdd_file.set_extension("bdd");

        let mut pp_logg = out_parent_folder.clone();
        pp_logg.push(&format!("{}_{}", core_filename.clone(), "pp_results"));
        pp_logg.set_extension("txt");

        let mut pruning_logg = out_parent_folder.clone();
        pruning_logg.push(&format!("{}_{}", core_filename.clone(), "pruning_logg"));
        pruning_logg.set_extension("txt");

        let mut logg_trace = out_parent_folder.clone();
        logg_trace.push(&format!("{}_{}", core_filename, "trace"));
        logg_trace.set_extension("txt");


        Self {
            out_parent_folder,
            bdd_file,
            pruning_logg,
            logg_trace,
            pp_logg,
        }
    }

    fn derive_core_filename(cipher_name: &str, num_rounds: usize, mode: DLmode, soft_lim: usize) -> String {
        // FIXME update how to do this!
        // FIXME copy of RawSoc make file name
        format!("{}_r{}_lim{}_mode_{}",
                cipher_name,
                num_rounds,
                soft_lim,
                mode,
        )
    }
}

/// The mode of operandi to use.
/// The available options are:
/// 1) Differential
/// 2) Linear.
#[derive(Debug, Clone)]
pub enum DLmode{
    Differential,
    Linear,
}

impl From<AnalysisMode> for DLmode {
    fn from(am: AnalysisMode) -> Self {
        use AnalysisMode::*;
        match am {
            Differential => DLmode::Differential,
            Linear => DLmode::Linear
        }
    }
}

impl From<DLmode> for AnalysisMode {
    fn from(dl: DLmode) -> Self {
        use DLmode::*;
        match dl {
            Differential => AnalysisMode::Differential,
            Linear => AnalysisMode::Linear
        }
    }
}

impl fmt::Display for DLmode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DLmode::{Linear, Differential};
        match self {
            Differential => write!(f, "diff")?,
            Linear => write!(f, "lin")?,
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Flags {
    /// Suppress logg data and results from being printed to stdout.
    /// Useful when some sort of progress bar is utilized.
    hide_output: bool,
    /// True if SolvedSoC is to be written to file
    write_soc: bool,

    write_meta: bool,

    mode: DLmode,
    /// Only "solve the SoC", meaning do not perform the post processing.
    /// Especially useful when all we want is to solve a specific soc for later processing.
    only_solve: bool,
}

impl Flags {
    #[inline]
    pub fn new(hide_output: bool, write_soc: bool, write_meta: bool, mode: DLmode, only_solve: bool) -> Self {
        Self {
            hide_output,
            write_soc,
            write_meta,
            mode,
            only_solve,
        }
    }

    /// Suppress logg data and results from being printed to stdout.
    /// Useful when some sort of progress bar is utilized.
    #[inline]
    pub fn hide_output(&self) -> bool {
        self.hide_output
    }

    /// True if SolvedSoC is to be written to file
    #[inline]
    pub fn write_soc(&self) -> bool {
        self.write_soc
    }

    /// True if extra metadata is to be written to file
    #[inline]
    pub fn write_meta(&self) -> bool {
        self.write_meta
    }

    /// Linear or Differential analysis?
    #[inline]
    pub fn mode(&self) -> DLmode {
        self.mode.clone()
    }

    /// Only "solve the SoC", meaning do not perform the post processing.
    /// Especially useful when all we want is to solve (and store) a specific soc for later processing.
    #[inline]
    pub fn only_solve(&self) -> bool {
        self.only_solve.clone()
    }
}


/// First stage of the DL process: The RawSoC is the SoC fresh out of the "factory", so to speak.
/// It is any SoC modelling its cipher "one-to-one", before any ops are performed on the SoC.
pub struct RawSoc<B, S>
    where
        B: BTHandler,
        S: SBoxHandler,
{
    setup: Setup,
    /// The SoC as it is "raw", meaning after construction from spec, before any ops are performed on it.
    soc: System,
    /// The matrix of the RawSoc's LHS's. They are easier to extract before we solve the SoC, even
    /// though we don't need it until the Processing of the SolvedSoC.
    lhss: Matrix,
    /// An overview over what Shards belongs to what round. As requested by the SimpleSolver.
    rounds: Vec<Vec<Id>>,
    /// An overview of which "interesting" linear combinations come from the same Shard, keyed
    /// by the origin Shard's Id. A linear combination is of "interest" if it is not absorbed as
    /// part of the solving process. See SimpleSolver documentation for more info.
    cohorts: HashMap<Id, Vec<Vob>>,
    bt_handler: B,
    sb_handler: S,
    ll_handler: Box<dyn LLHandler>,
}

impl<B, S> RawSoc<B, S>
    where
        B: BTHandler + Clone + Debug,
        S: SBoxHandler,
{

    pub fn solve_soc(self, setup: &Setup, progress: Progress) -> SolvedSoC<B, S, Progress>
    {
        // todo document hidden assumptions
        let mut solver = SimpleSolver::new(
            self.soc,
            self.rounds,
            Id::new(0), // TODO remove
            self.cohorts.clone(),
            self.ll_handler.block_size(0),
            progress.clone(),
        );
        solver.run(setup.soft_lim());

        let SolverResultOk {
            librarian,
            master,
            step,
            active_area
        }
            = solver.finalize();

        // == Write Shard to .bdd file ==
        let out_setup = setup.out_files();

        // Create parent folder if it does not exist
        // TODO better error handling
        let _ = fs::DirBuilder::new()
            .recursive(true)
            .create(out_setup.out_parent_folder.clone());

        crush::soc::utils::print_system_to_file(&master, &out_setup.bdd_file);


        SolvedSoC {
            setup: (*setup).clone(),
            soc: master,
            lhss: self.lhss,
            cohorts: self.cohorts,
            bt_handler: self.bt_handler,
            sb_handler: self.sb_handler,
            ll_handler: self.ll_handler,
            active_area,
            step: NonZeroUsize::new(step).unwrap(),
            loggs: Loggers {prune_logger: Some(librarian), process_result: None },
        }
    }

    /// Creates a String based on the name of the Cipher, the number of rounds and the soft_lim used.
    /// I.e. perhaps the main setup parameters, and may be useful for identifications of run-through.
    ///
    /// Note that that if this is to be used as a file name, then the caller must ensure that a file
    /// extension is set.
    fn make_file_name(setup: &Setup, cipher_name: &str) -> String {
        format!("{}_r{}_lim{}_mode{}",
                cipher_name,
                setup.num_rounds(),
                setup.soft_lim(),
                setup.dl_mode(),
        )
    }

}

pub struct SolvedSoC<B, S, F>
    where
        B: BTHandler + Clone + Debug,
        S: SBoxHandler,
        F: SPFactory + Clone,
{
    /// Various settings
    setup: Setup,
    /// The SoC as it is "raw", meaning after construction from spec, before any ops are performed on it.
    soc: System,
    /// The matrix of the RawSoc's LHS's. They are easier to extract before we solve the SoC, even
    /// though we don't need it until the Processing of the SolvedSoC.
    lhss: Matrix,
    /// An overview of which "interesting" linear combinations come from the same Shard, keyed
    /// by the origin Shard's Id. A linear combination is of "interest" if it is not absorbed as
    /// part of the solving process. See SimpleSolver documentation for more info.
    cohorts: HashMap<Id, Vec<Vob>>,
    bt_handler: B,
    sb_handler: S,
    ll_handler: Box<dyn LLHandler>,
    active_area: Range<usize>,
    step: NonZeroUsize,
    loggs: Loggers<F>,
}

impl<B, S, F> SolvedSoC<B, S, F>
    where
        B: BTHandler + Clone + Debug,
        S: SBoxHandler,
        F: SPFactory + Clone,
{

    pub fn analyse<P>(mut self, factory_arena: P) -> IoResult<ProcessedResult>
        where
            P: PPFactory,
    {
        // Make the SoC into a single Shard
        let master: HashMap<Id, RefCell<Shard>> = self.soc.drain_bdds().collect();
        debug_assert_eq!(master.len(), 1);
        let (_master_id, master_cell) = master.into_iter().next().unwrap();
        let master = master_cell.into_inner();

        // Assuming bits in == bits out
        let beta_len = self.sb_handler.num_sboxes(0)*self.step.get();


        // ======================== PPv5 ===================================
        let beta_depth = self.active_area.end - beta_len;
        let alpha_depth = self.active_area.start;
        let master_md = SolvedSocMeta::new(self.active_area,
                                           self.step,
                                           alpha_depth,
                                           beta_depth);

        let handlers = Handlers::new(self.bt_handler.clone(), self.sb_handler);


        let tx = TraceLogger::init_and_run(self.setup.out_files.logg_trace.clone());

        let pp_res = start_post_processing(master,
                                           master_md,
                                           self.lhss,
                                           handlers,
                                           factory_arena,
                                           format!("{}_r{}",
                                                   self.setup.cipher_name,
                                                   self.bt_handler.nr_of_rounds(),

                                           ),
                                           tx,
                                           self.setup.mode.clone().into()

        );
        // ======================== PPv5 Done================================

        let out_setup = self.setup.out_files();

        // Create parent folder if it does not exist
        // TODO better error handling
        let _ = fs::DirBuilder::new()
            .recursive(true)
            .create(out_setup.out_parent_folder.clone());

        Self::write_file(out_setup.pp_logg.clone(),
                         &DisplayResult::AsSummary(&pp_res).to_string()
        ).expect("Couldn't write to file");
        // Self::write_file(file_path, &self.loggs.write_result()
        //     .expect("Writing to String shouldn't fail")
        // ).expect("Couldn't write to file");

            // FIXME
            // // Write meta data, if flag is set
            // if flags.write_meta {
            //     file_path_meta.push(format!("{}{}{}", name, "_results_meta_", match flags.mode {
            //         DLmode::Differential => "diff",
            //         DLmode::Linear => "lin",
            //     }));
            //     file_path_meta.set_extension("txt");
            //
            //     //FIXME meta output currently disabled
            //     // println!("Meta output currently disabled");
            //
            //     // Self::write_file(file_path_meta, &self.loggs.write_meta()
            //     //     .expect("Writing to String shouldn't fail")
            //     // )?;
            // }

        Ok(pp_res)
    }




    fn write_file(pathbuf: PathBuf, content: &str) -> IoResult<()> {
        let write_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(pathbuf)
            .expect("Couldn't open file");
        let mut writer = BufWriter::new(&write_file);
        writeln!(writer, "{}", content)?;
        Ok(())
    }

}

struct Loggers<F>
    where
        F: SPFactory + Clone,
{
    prune_logger: Option<Librarian<F>>,
    process_result: Option<ProcessedResult>,
}

impl<F> Loggers<F>
    where
        F: SPFactory + Clone,
{

    fn new() -> Self {
        Self {
            prune_logger: None,
            process_result: None
        }
    }

    fn write_result(&self) -> Result<String, FmtError> {
        let mut buff = String::new();

        if self.process_result.is_none() {
            panic!("We got a None when we should have a Some!");
        }
        writeln!(buff, "{}", DisplayResult::AsSummary(self.process_result.as_ref().unwrap())
        )?;
        Ok(buff)
    }

    fn write_meta(&self)  -> Result<String, FmtError> {
        let mut buff = String::new();

        if self.process_result.is_none() {
            panic!("We got a None when we should have a Some!");
        }
        // FIXME missing!
        // writeln!(buff, "{}", self.process_result.as_ref().unwrap().print_meta()
        //     .expect("Writing to String shouldn't fail")
        // )?;
        Ok(buff)

    }


    fn write_prune_logg(&self) -> Result<String, FmtError> {
        let mut buff = String::new();

        if self.prune_logger.is_none() {
            panic!("We got a None when we should have a Some!");
        }

        writeln!(buff, "Dummy implementation!")
            .expect("Writing to String shouldn't fail");

        Ok(buff)
    }

}