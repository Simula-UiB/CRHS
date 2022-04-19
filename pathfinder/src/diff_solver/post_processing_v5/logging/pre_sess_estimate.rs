use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Result as FmtResult;
use std::iter::FromIterator;

use console::style;

use crush::soc::bdd::differential::Depth;
use crush::soc::bdd::differential::wd::{NWDistribution, WDLevel, WDPresence};
use crush::soc::bdd::Bdd as Shard;
use crush::soc::Id;

use crate::diff_solver::post_processing_v5::SolvedSocMeta;
use std::ops::Range;

// ************************************* Contents **************************************************
// struct PreSessEstimateMD
// enum WDKind
// struct AlphaCandidatesMD
// enum DisplayPreSessEstimateMD<'a>
// - impl fmt::Display for DisplayPreSessEstimateMD<'_>
//
// struct MessConsSummary
// struct MasterLayoutMD
// *********************************** Contents End ************************************************

// TODO update fmt functions to respect format params, like width, fill and alignment

// todo reformat. Newer MD structs does not fit into this mold, but still fits under the name.
// Ex: MessConSummary and MasterLayoutMD
/// Extracts and keeps metadata, primarily the kind related to what happens in the analysis pre finding
/// the best SESS estimate.
#[derive(Debug, Clone)]
pub struct PreSessEstimateMD {
    kind: WDKind,
    num_nodes: usize,
    weights_len_and_count: BTreeMap<usize, u32>,
    weights_seen_and_count: BTreeMap<u32, u32>,
    lowest_seen: u32,
    nt_lew: Option<u32>,
    highest_seen: u32,
    median: u32,
    sum: u32,
}

#[derive(Debug, Clone)]
enum WDKind {
    /// WDs built from source node (Sierra) to nodes on Alpha level.
    /// Depth of Alpha Level
    SierraAlpha(Depth),
    /// WDs build from sink node (Tau) to nodes on Alpha level
    /// Depth of Alpha Level
    TauAlpha(Depth),
    /// WDs build from sink node (Tau) to nodes on Beta level
    /// Depth of Beta Level
    TauBeta(Depth),
    /// WDs build from nodes on Alpha level to nodes on Beta level. This level is likely to be
    /// built using a TargetedFactory, meaning that distributions are built from some 'targeted'
    /// nodes in the Alpha level.
    /// Values are 'Depth of Beta Level' and number of Alpha nodes passed as targets to the TargetedFactory
    AlphaBeta(Depth, usize),

    AlphaCandidates(AlphaCandidatesMD),

    MessConSummary(MessConsSummary),
}

#[derive(Debug, Clone)]
struct AlphaCandidatesMD {
    alpha_level_nt_lew: u32,
    count_plus_zero: usize,
    count_plus_one: usize,
    count_plus_two: usize,
}

#[derive(Debug, Clone)]
struct MessConsSummary {
    alpha_level_len: usize,
    beta_level_len: usize,
    lowest_nr_cons: usize,
    second_lowest_nr: Option<usize>,
    highest_nr_cons: usize,
    sum_actual_cons: usize,
    median: usize,
    avg: f64,
}

// =================================================================================================
// =================================================================================================
// =================================================================================================

impl AlphaCandidatesMD {
    fn analyse_candidates<N>(alpha_level_nt_lew: u32, candidates: &Vec<Vec<(Id, N)>>) -> Self
        where
            N: NWDistribution
    {
        Self {
            alpha_level_nt_lew,
            count_plus_zero: candidates[0].len(),
            count_plus_one: candidates[1].len(),
            count_plus_two: candidates[2].len(),
        }
    }

    fn fmt_log_entry(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        writeln!(f, "\nNumber of nodes | with nt-lew:")?;
        writeln!(f, "{: >15} | {: >4}", self.alpha_level_nt_lew, self.count_plus_zero)?;
        writeln!(f, "{: >15} | {: >4}", self.alpha_level_nt_lew+1, self.count_plus_one)?;
        writeln!(f, "{: >15} | {: >4}", self.alpha_level_nt_lew+2, self.count_plus_two)?;
        Ok(())
    }
}

// =================================================================================================
// =================================================================================================
// =================================================================================================

impl PreSessEstimateMD {
    pub fn analyse_sierra_alpha<N: NWDistribution> (sierra_alpha: &WDLevel<N>, md: &SolvedSocMeta) -> Self {
        Self::extract_metadata(&sierra_alpha, WDKind::SierraAlpha(md.alpha_lvl_depth))
    }


    pub fn analyse_tau_alpha<N: NWDistribution> (tau_alpha: &WDLevel<N>, md: &SolvedSocMeta) -> Self
    {
        Self::extract_metadata(tau_alpha, WDKind::TauAlpha(md.alpha_lvl_depth))
    }


    pub fn analyse_tau_beta<N: NWDistribution> (tau_alpha: &WDLevel<N>, md: &SolvedSocMeta) -> Self
    {
        Self::extract_metadata(tau_alpha, WDKind::TauBeta(md.beta_lvl_depth))
    }


    pub fn analyse_alpha_beta<N: NWDistribution> (alpha_beta: &WDLevel<N>,
                                 md: &SolvedSocMeta,
                                 number_of_alpha_targets: usize) -> Self
    {
        Self::extract_metadata(alpha_beta, WDKind::AlphaBeta(md.beta_lvl_depth, number_of_alpha_targets))
    }


    pub fn analyse_alpha_candidates<N: NWDistribution>(alpha_level_nt_lew: u32, candidates: &Vec<Vec<(Id, N)>>) -> Self
    {
        Self::extract_metadata(
            &(WDLevel::from_iter(candidates
                .clone()
                .into_iter()
                .flat_map(|vec| vec.into_iter()) )
            ),
            WDKind::AlphaCandidates(
                AlphaCandidatesMD::analyse_candidates(alpha_level_nt_lew, candidates)
            ),
        )
    }

    pub fn analyse_mess_cons(alpha_beta: &WDLevel<WDPresence>,
                             alpha_level_len: usize,
                             beta_level_len: usize, )
                             -> Self
    {
        Self::extract_metadata(
            alpha_beta,
            WDKind::MessConSummary(MessConsSummary::new(alpha_beta, alpha_level_len, beta_level_len))
        )
    }


    fn extract_metadata<N>(level: &WDLevel<N>, kind: WDKind) -> Self
        where
            N: NWDistribution,
    {
        let num_nodes = level.len();

        let weights_len_and_count = level.iter()
            .map(|d| d.1.existing_weights().len())
            .fold(BTreeMap::new(),
                  |mut acc, len| {
                      let l = acc.entry(len).or_insert(0);
                      *l += 1;
                      acc
                  });

        let mut all_weights: Vec<u32> = level.iter()
            .map(|d| d.1.existing_weights())
            .fold(Vec::new(),
                  |mut acc, bt| {
                      let t: Vec<u32> = bt.into_iter().collect();
                      acc.extend_from_slice(&t);
                      acc
                  } );

        all_weights.sort_unstable();
        let lowest_seen = all_weights.get(0).unwrap().clone();
        let highest_seen = all_weights.get(all_weights.len() - 1).unwrap().clone();
        let median = all_weights.get(all_weights.len() / 2).unwrap().clone();
        let sum = all_weights.iter().sum();
        let mut nt_lew = None;
        for candidate in all_weights.iter() {
            if candidate != &0 {
                nt_lew = Some(*candidate);
                break;
            }
        }

        let weights_seen_and_count = all_weights.into_iter()
            .fold(BTreeMap::new(),
                  |mut acc, weight| {
                      let c = acc.entry(weight).or_insert(0);
                      *c += 1;
                      acc
                  });

        Self {
            kind,
            num_nodes,
            weights_len_and_count,
            weights_seen_and_count,
            lowest_seen,
            nt_lew,
            highest_seen,
            median,
            sum,
        }
    }

    fn fmt_log_entry(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        use WDKind::*;
        let width = 100;

        match self.kind {
            SierraAlpha(depth) => {
                writeln!(f, "\n{:-^w$}", " Weight Distribution MetaData for Sierra Alpha->Paths ", w=width)?;
                writeln!(f, "Alpha at Depth: {}", depth)?;
                writeln!(f, "Nodes on Alpha level: {}", self.num_nodes)?;
                self.fmt_log_entry_core(f)
            },
            TauAlpha(depth) => {
                writeln!(f, "\n{:-^w$}", " Weight Distribution MetaData for Tau->Alpha Paths ", w=width)?;
                writeln!(f, "Alpha at Depth: {}", depth)?;
                writeln!(f, "Nodes on Alpha level: {}", self.num_nodes)?;
                self.fmt_log_entry_core(f)
            },
            TauBeta(depth) => {
                writeln!(f, "\n{:-^w$}", " Weight Distribution MetaData for Tau->Beta Paths ", w=width)?;
                writeln!(f, "Beta at Depth: {}", depth)?;
                writeln!(f, "Nodes on Beta level: {}", self.num_nodes)?;
                self.fmt_log_entry_core(f)
            },
            AlphaBeta(depth, num_targets) => {
                writeln!(f, "\n{:-^w$}", " Weight Distribution MetaData for Alpha->Beta Paths ", w=width)?;
                writeln!(f, "Beta at Depth: {}", depth)?;
                writeln!(f, "Nodes on Beta level: {}", self.num_nodes)?;
                writeln!(f, "Number of Alpha nodes passed as targets to the TargetedFactory: {}", num_targets)?;
                self.fmt_log_entry_core(f)
            },
            AlphaCandidates(ref ac_md) => {
                writeln!(f, "\n{:-^w$}", " Metadata Alpha Candidates ", w=width)?;
                writeln!(f, "Number of candidates: {}", self.num_nodes)?;
                ac_md.fmt_log_entry(f)?;
                self.fmt_log_entry_core(f)
            },
            MessConSummary(ref msc) => {
                msc.print_log_file(f)
                // OBS: calling self.fmt_log_entry_core(...) is unnecessary, as that part of self is
                // created based on the alpha->beta level, which is already covered elsewhere.
            }
        }
    }


    fn fmt_log_entry_core(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
//         write!(f, "This section deals with metadata regarding the paths from the source at Depth 0,\n\
// to the nodes at the Alpha level, Depth {}. These paths are known as the Alpha paths, and represents\n\
// the input difference/characteristic to the cipher. These data are extracted to confirm or bust a theory,\n\
// but if you see them, then they have been left here for anyone who may find them interesting\n\
// The data deals mostly with the presence/absence of weighs in a node.\n\n",
//         self.kind)?;

        writeln!(f, "\nEach distribution will have a set number weights that have 1 or more paths.\n\
        Below we see how many nodes share the same number of unique weights, in terms of active S-boxes.")?;
        writeln!(f, " Unique weights | Observed in _ number of nodes")?;
        for (len, c) in self.weights_len_and_count.iter() {
            writeln!(f, "{: >15} | {: >3}", len, c)?;
        }

        writeln!(f, "\nNext we get an overview of how many times a weight, in terms of active S-boxes,\n\
        has been observed across all the given weight distributions:")?;
        writeln!(f, "Observed weight | Number of times")?;
        for (w, c) in self.weights_seen_and_count.iter() {
            writeln!(f, "{: >15} | {: >3}", w, c)?;
        }

        writeln!(f, "\nWeights seen:\
\n{: >4}{: <19} {: >3}\
\n{: >4}{: <19} {: >3}\
\n{: >4}{: <19} {: >3}\
\n{: >4}{: <19} {: >3}\
\n{: >4}{: <19} {: >3.2}",
            "", "Lowest:", self.lowest_seen,
            "", "Lowest non trivial:", match self.nt_lew {
                None => "None".to_string(),
                Some(num) => format!("{}", num),
            },
            "", "Highest:", self.highest_seen,
            "", "Median:", self.median,
            "", "Avg:",
            // self.sum as f64 / self.weights_len_and_count.values().cloned().sum::<u32>() as f64,
            self.sum as f64 / self.weights_len_and_count.iter()
                .map(|(w,c)| *w as u32 * c).sum::<u32>() as f64,
        )?;

        Ok(())
    }

    fn fmt_report(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        // todo add header?
        self.fmt_log_entry(f)
    }
}

// =================================================================================================
// ==================================== MESSCons Summary ===========================================
// =================================================================================================

impl MessConsSummary {
    fn new(alpha_beta: &WDLevel<WDPresence>, alpha_level_len: usize, beta_level_len: usize)
        -> MessConsSummary
    {

        // Get each Start Nodes' number of end connections
        let mut actual_connections: Vec<usize> =
            alpha_beta.iter()
                // s is a MESS, &WDPresence contains an overview of the end nodes
                .map(|s| s.1.end_connections().len())
                .collect();

        actual_connections.sort();
        let median = actual_connections.get(actual_connections.len() / 2).unwrap();
        let sum: usize = actual_connections.iter().sum();

        let lowest_nr_cons = actual_connections[0];
        let mut second_lowest_nr = None;
        for nr in actual_connections.iter() {
            if *nr != lowest_nr_cons {
                second_lowest_nr = Some(*nr);
                break;
            }
        }

        MessConsSummary {
            alpha_level_len,
            beta_level_len,
            lowest_nr_cons,
            second_lowest_nr,
            highest_nr_cons: actual_connections[actual_connections.len() - 1],
            sum_actual_cons: sum,
            median: *median,
            avg: sum as f64 / actual_connections.len() as f64,
        }
    }

    fn print_log_console(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        let width = 100;

        // MESS Connections Summary
        writeln!(f, "{:=^w$}", style(" MESSCon's Summary: ").cyan(), w=width)?;
        writeln!(f, "{: ^w$}", style(" Multiple End Nodes - Single Start Node Connection's: ").cyan(), w=width)?;
        writeln!(f, "Alpha level has {} SCon nodes.", self.alpha_level_len)?;
        writeln!(f, "Beta level has {} ECon nodes.", self.beta_level_len)?;
        let mul = (self.alpha_level_len * self.beta_level_len) as f64;
        writeln!(f, "This yields a maximum of {} possible SESS Connections.", mul)?;


        writeln!(f, "\nGot {} SESS Connections, ({: >2.2}% of max possible):",
                 style(self.sum_actual_cons).yellow(), self.sum_actual_cons as f64 * 100_f64 / mul)?;
        writeln!(f, "Lowest nr of SESS Connections for a start node: {}", self.lowest_nr_cons)?;
        writeln!(f, "Second lowest nr of SESS Connections for a start node: {}",
                 match self.second_lowest_nr {
                     Some(nr) => nr.to_string(),
                     None => "None".to_string(),
                 }
        )?;
        writeln!(f, "Highest nr of SESS Connections for a start node: {}", self.highest_nr_cons)?;
        writeln!(f, "Median nr of SESS Connections for a start node: {}", self.median)?;
        writeln!(f, "Average nr of SESS Connections for a start node: {: >2.2}", self.avg)?;

        Ok(())
    }

    fn print_log_file(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        let width = 100;

        // MESS Connections Summary
        writeln!(f, "{:=^w$}", " MESSCon's Summary: ", w=width)?;
        writeln!(f, "{: ^w$}", " Multiple End Nodes - Single Start Node Connection's: ", w=width)?;
        writeln!(f, "Alpha level has {} SCon nodes.", self.alpha_level_len)?;
        writeln!(f, "Beta level has {} ECon nodes.", self.beta_level_len)?;
        let mul = (self.alpha_level_len * self.beta_level_len) as f64;
        writeln!(f, "This yields a maximum of {} possible SESS Connections.", mul)?;


        writeln!(f, "\nGot {} SESS Connections, ({: >2.2}% of max possible):",
                 self.sum_actual_cons, self.sum_actual_cons as f64 * 100_f64 / mul)?;
        writeln!(f, "Lowest nr of SESS Connections for a start node: {}", self.lowest_nr_cons)?;
        writeln!(f, "Second lowest nr of SESS Connections for a start node: {}",
                 match self.second_lowest_nr {
                     Some(nr) => nr.to_string(),
                     None => "None".to_string(),
                 }
        )?;
        writeln!(f, "Highest nr of SESS Connections for a start node: {}", self.highest_nr_cons)?;
        writeln!(f, "Median nr of SESS Connections for a start node: {}", self.median)?;
        writeln!(f, "Average nr of SESS Connections for a start node: {: >2.2}", self.avg)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MasterLayoutMD {
    /// Start and end Depths of Master
    depth_range: Range<usize>,
    alpha_level: Depth,
    alpha_width: usize,
    beta_level: Depth,
    beta_width: usize,
    /// Depth of the widest level in master
    widest_level: Depth,
    /// Number of nodes of the widest level in master
    widest_level_width: usize,
    /// Depth of the *second* widest level in master
    s_widest_level: Depth,
    /// Number of nodes of the *second* widest level in master
    s_widest_level_width: usize,
    /// Total number of nodes in master, including the sink node
    tot_node_count: usize,
    /// Width and depth of the levels in master, sorted by width.
    levels_width: BTreeMap<usize, Vec<Depth>>,
}

impl MasterLayoutMD {
    pub fn new(master: &Shard, master_md: &SolvedSocMeta) -> MasterLayoutMD {
        let full_range = Range{start: 0, end: master.get_sink_level_index()};
        let (widest, s_widest) = master.widest_levels(&full_range);
        let tot_node_count = master.get_size();
        let alpha_width = master.level(master_md.alpha_lvl_depth).unwrap().get_nodes_len();
        let beta_width = master.level(master_md.beta_lvl_depth).unwrap().get_nodes_len();

        let ten_widest: BTreeMap<usize, Vec<Depth>> = master.iter_levels().enumerate()
            .map(|(depth, level)| (level.get_nodes_len(), depth))
            .fold(
                BTreeMap::default(),
                |mut acc, (width, depth)| {

                    let ds = acc.entry(width).or_insert(Vec::new());
                    ds.push(depth);
                    acc
                }
            );

        Self {
            depth_range: full_range,
            alpha_level: master_md.alpha_lvl_depth,
            alpha_width,
            beta_level: master_md.beta_lvl_depth,
            beta_width,
            widest_level: widest.0,
            widest_level_width: widest.1,
            s_widest_level: s_widest.0,
            s_widest_level_width: s_widest.1,
            tot_node_count,
            levels_width: ten_widest,
        }
    }

    /// Returns the Depth and width of the widest and second widest levels.
    pub fn get_widest_levels(&self) -> ((usize, usize), (usize, usize)) {
        ((self.widest_level,self.widest_level_width), (self.s_widest_level,self.s_widest_level_width))
    }

    fn fmt_log_entry(&self, f: &mut fmt::Formatter) -> FmtResult {
        let width = 100;

        writeln!(f, "\n{:-^w$}\n", " Master Shard Levels MD ", w = width)?;
        writeln!(f, "Master Shard has {} nodes distributed over {} levels, including sink node and level.\n",
                 self.tot_node_count ,self.depth_range.end + 1)?;
        writeln!(f, "Alpha level is at Depth {: >3}, and contains {: >4} nodes.", self.alpha_level, self.alpha_width)?;
        writeln!(f, " Beta level is at Depth {: >3}, and contains {: >4} nodes.", self.beta_level, self.beta_width)?;
        writeln!(f, " Sink level is at Depth {: >3}.", self.depth_range.end)?;

        writeln!(f, "\n       Widest level is at Depth {}, and contains {} nodes.", self.widest_level, self.widest_level_width)?;
        writeln!(f, "Second widest level is at Depth {}, and contains {} nodes.", self.s_widest_level, self.s_widest_level_width)?;

        writeln!(f, "\nThe top ten widest levels in Master:\n\
        (Note that we count top down. This means that we may miss the above mentioned widest, as\n\
        that one was found searching from the bottom and up).")?;
        writeln!(f, "\nWidth | Depth(s)")?;
        for (width, depths) in self.levels_width.iter().rev().take(10) {
            write!(f, " {: >4} | ", width)?;
            for depth in depths.iter() {
                write!(f, "{: >3}, ", depth)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

// =================================================================================================
// =================================================================================================
// =================================================================================================

pub enum DisplayPreSessEstimateMD<'a> {
    AsLogEntry(&'a PreSessEstimateMD),
    AsReport(&'a PreSessEstimateMD),
    MlAsLogEntry(&'a MasterLayoutMD),
}

impl fmt::Display for DisplayPreSessEstimateMD<'_> {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        use DisplayPreSessEstimateMD::*;

        match self {
            AsLogEntry(wdp) => { wdp.fmt_log_entry(f)}
            AsReport(wdp) => { wdp.fmt_report(f)}
            MlAsLogEntry(ml) => {ml.fmt_log_entry(f)}
        }
    }
}