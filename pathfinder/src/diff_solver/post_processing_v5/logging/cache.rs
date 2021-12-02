use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::sync::mpsc::Sender;

use crush::soc::bdd::Bdd as Shard;
use crush::soc::bdd::differential::wd::{NWDistribution, TargetedFactory, TransparentFactory, WDCountV2, WDLevel, WDPresence, NcWDistribution};
use crush::soc::Id;

use crate::diff_solver::post_processing_v5::logging::{PreSessEstimateMD as PreMD, PreSessEstimateMD, MasterLayoutMD, AlphaBetaInnerPaths};
use crate::diff_solver::post_processing_v5::{SolvedSocMeta};
use crush::soc::bdd::differential::PPFactory;
use crate::diff_solver::post_processing_v5::SessEstimate;
use crate::diff_solver::post_processing_v5::hull_calc::{ProcessedResultSection};


#[derive(Debug, Clone)]
pub enum LogType
{
    PreSessEstimateMDEntry(PreSessEstimateMD),
    SessEstimate(SessEstimate),
    LayoutMaster(MasterLayoutMD),
    ResultSection(ProcessedResultSection),
    AlphaBetaInnerPaths(AlphaBetaInnerPaths),
}

/// This is a cache/logg hybrid.
/// 1) It's a cache, with respect to caching metadata used later in the post process.
/// 2) It's a logger, with respect to the fact that this metadata is later used to generate an end report
/// for the user to see. It also offers the functionality of sending "packets" of the metadata it collects
/// as they become available. This may allow an end user to perform more advanced analysis on the metadata,
/// or to simply write them to a file. This last one is especially useful if a panic is caused for
/// some reason. The TraceLogger, part of this module, achieves this last functionality.
///
/// It has a last, "half", purpose as well:
/// 0) Some of the data creation has been moved inside this cache. This serves the purpose of de-cluttering
/// the "main" body of code, allowing it to minimize the lines of code which is not directly related to the
/// analysis, thus increasing readability.
pub struct Cache
{
    /// The logger which we are to pass on relevant data to.
    trace_out: Sender<LogType>,
    master_md: SolvedSocMeta,
    pre_sess_logs: RefCell<Vec<PreSessEstimateMD>>,
    alpha_level_nt_lew: Cell<u32>,
    num_alpha_candidates: Cell<usize>,
    sess_estimate: RefCell<Option<SessEstimate>>,
    master_layout_md: RefCell<Option<MasterLayoutMD>>,
    res_sections: RefCell<Vec<ProcessedResultSection>>,
    /// Paths between sierra->alpha, tau->beta, and alpha->beta
    abi_md: RefCell<Option<AlphaBetaInnerPaths>>,
}

impl Cache {
    /// Creates a new Cache
    pub fn new(tracer_channel: Sender<LogType>, master_md: SolvedSocMeta) -> Self {
        Self {
            trace_out: tracer_channel,
            master_md,
            pre_sess_logs: RefCell::new(vec![]),
            alpha_level_nt_lew: Cell::new(0),
            num_alpha_candidates: Cell::new(0),
            sess_estimate: RefCell::new(None),
            master_layout_md: RefCell::new(None),
            res_sections: RefCell::new(vec![]),
            abi_md: RefCell::new(None)
        }
    }

    pub fn master_md(&self) -> &SolvedSocMeta {
        &self.master_md
    }

    pub fn alpha_level_nt_lew(&self) -> Option<u32> {
        match self.alpha_level_nt_lew.get() {
            0 => None,
            n => Some(n),
        }
    }

    pub fn number_of_alpha_candidates(&self) -> Option<usize> {
        match self.num_alpha_candidates.get() {
            0 => None,
            n => Some(n),
        }
    }

    /// Returns the sum of all paths found in the SESS Estimate. If the SESS estimate is missing, then
    /// None is returned. Otherwise a Some value with the result of the overflowing_add op is returned.
    /// If an overflow would have occurred then the wrapped value is returned.
    pub fn sum_sess_est_inner_paths(&self) -> Option<(usize, bool)> {
        let maybe_sess = self.sess_estimate.borrow();
        match *maybe_sess {
            None => None,
            Some(ref est) => {
                Some(est.hull_distribution().unwrap().total_number_of_paths_overflowing())
            }
        }
    }

}

// =================================================================================================
// ========================================= More Impls ============================================
// =================================================================================================

impl Cache
{
    /// Extracts data and metadata from the sierra -> alpha distributions (== the distributions from
    /// source node to the alpha level, equivalent to the input variables to the cipher).
    /// Relevant data are sent to the PreSessEstimateMD function of the same name, before logging that
    /// result to the logger.
    pub fn make_and_analyse_sierra_alpha<P>(&self, master: &Shard, progress: &P)
        where P: PPFactory,
    {
        // Obs, end is alpha lvl + 1, as "...for_level_top_bottom(...) assumes end is below target depth,
        // but does not use end for any other purpose.
        let top_active_area = Range{start: 0, end: self.master_md.alpha_lvl_depth+1};

        // WDLevel from the Source (depth 0) to alpha level (depth 'n'). n is expected to be the size
        // of the 'input', or plaintext, and the area from source to n is expected to adhere to the
        // invariants for an active area. For a complete S-box layered cipher, I know that the
        // invariants will be upheld, but I need to look closer into what happens when the S-box layer
        // is incomplete. Thus, incomplete S-box layers may yield wrong results.
        let sierra_alpha: WDLevel<WDCountV2> = master.weight_distributions_for_level_top_bottom(
            self.master_md.alpha_lvl_depth,
            &top_active_area,
            self.master_md.step.clone(),
            &TransparentFactory::new(),
            progress,
        );

        let log = PreMD::analyse_sierra_alpha(&sierra_alpha, &self.master_md);
        self.pre_sess_logs.borrow_mut().push(log.clone());

        let _ = self.trace_out.send(LogType::PreSessEstimateMDEntry(log));

    }


    pub fn make_and_analyse_tau_alpha<W: NWDistribution>(&self, master: &Shard) -> WDLevel<W> {
        let alpha_level = master.weight_distributions_for_level (
            self.master_md.alpha_lvl_depth,
            &self.master_md.active_area,
            self.master_md.step.clone(),
            &TransparentFactory::new(),
        );

        let log = PreMD::analyse_tau_alpha(&alpha_level, &self.master_md);
        self.pre_sess_logs.borrow_mut().push(log.clone());

        let _ = self.trace_out.send(LogType::PreSessEstimateMDEntry(log));

        alpha_level
    }



    pub fn analyse_alpha_candidates<W: NWDistribution>(&self, alpha_level_nt_lew: u32, alpha_candidates: &Vec<Vec<(Id, W)>>) {
        self.alpha_level_nt_lew.set(alpha_level_nt_lew);

        let logg = PreMD::analyse_alpha_candidates(alpha_level_nt_lew, alpha_candidates);
        self.pre_sess_logs.borrow_mut().push(logg.clone());

        let _ = self.trace_out.send(LogType::PreSessEstimateMDEntry(logg));
    }


    pub fn make_and_analyse_tau_beta<W: NWDistribution>(&self, master: &Shard) -> WDLevel<W> {
        let beta_level = master.weight_distributions_for_level
        (
            self.master_md.beta_lvl_depth,
            &self.master_md.active_area,
            self.master_md.step.clone(),
            &TransparentFactory::new(),
        );

        let logg = PreMD::analyse_tau_beta(&beta_level, &self.master_md);

        self.pre_sess_logs.borrow_mut().push(logg.clone());
        let _ = self.trace_out.send(LogType::PreSessEstimateMDEntry(logg));

       beta_level
    }


    pub fn make_and_analyse_alpha_beta<W, P>(&self, master: &Shard, targets: Vec<Id>, progress: &P)
        -> WDLevel<W>
        where W: NWDistribution, P: PPFactory,
    {
        let target_len = targets.len();
        self.num_alpha_candidates.set(target_len);

        let alpha_beta = master.weight_distributions_for_level_top_bottom
        (
            self.master_md.beta_lvl_depth,
            &self.master_md.active_area,
            self.master_md.step.clone(),
            &TargetedFactory::new(targets),
            progress,
        );

        let logg = PreMD::analyse_alpha_beta(
            &alpha_beta,
            &self.master_md,
            target_len);

        self.pre_sess_logs.borrow_mut().push(logg.clone());
        let _ = self.trace_out.send(LogType::PreSessEstimateMDEntry(logg));

        alpha_beta
    }


    /// Extracts some metadata with regards to how nodes on alpha level are connected with nodes on
    /// the beta level.
    pub fn analyse_mess_cons<P: PPFactory>(&self, master: &Shard, alpha_level_width: usize, progress: &P) {

        // This alfa beta differs from the one in make_and_analyse_alpha_beta in that it does not
        // target any nodes on alpha, and it uses WDPresence instead of WDCountV2 as its distribution.
        // These two changes hopefully counteracts one another in terms of memory consumption during
        // construction. (Construction us usually where any memory issues occur, the finished product
        // tends to be quite manageable. This is due to the fact that the widest levels tend to be
        // in between alpha and beta somewhere, and may be many times the size of at least alpha).
        let alpha_beta: WDLevel<WDPresence> = master.weight_distributions_for_level_top_bottom
        (
            self.master_md.beta_lvl_depth,
            &self.master_md.active_area,
            self.master_md.step.clone(),
            &TransparentFactory::new(),
            progress
        );

        let logg = PreMD::analyse_mess_cons(&alpha_beta,
                                            alpha_level_width,
                                            alpha_beta.len());
        self.pre_sess_logs.borrow_mut().push(logg.clone());
        let _ = self.trace_out.send(LogType::PreSessEstimateMDEntry(logg));
    }



    pub fn register_sess_estimate(&self, estimate: SessEstimate) {
        *self.sess_estimate.borrow_mut() = Some(estimate.clone());
        let _ = self.trace_out.send(LogType::SessEstimate(estimate));
    }

    /// Extracts some metadata about Master, its levels and their widths.
    /// Returns the Depth and width of the widest and second widest levels.
    pub fn record_master_layout(&self, master: &Shard)
        -> ((usize, usize), (usize, usize)) {
        let log = MasterLayoutMD::new(master, self.master_md());
        let res = log.get_widest_levels();
        *self.master_layout_md.borrow_mut() = Some(log.clone());

        let _ = self.trace_out.send(LogType::LayoutMaster(log));
        res
    }

    pub fn record_processed_result_section(&self, section: ProcessedResultSection) {

        self.res_sections.borrow_mut().push(section.clone());
        let _ = self.trace_out.send(LogType::ResultSection(section));
    }

    pub fn record_abi_md(&self, abi: AlphaBetaInnerPaths) {
        *self.abi_md.borrow_mut() = Some(abi.clone());
        let _ = self.trace_out.send(LogType::AlphaBetaInnerPaths(abi));
    }
}


