pub use cache::{Cache, LogType};
pub use pre_sess_estimate::{DisplayPreSessEstimateMD, PreSessEstimateMD, MasterLayoutMD};
pub use trace_logger::TraceLogger;

use std::fmt;
use std::fmt::Result as FmtResult;

mod pre_sess_estimate;
mod cache;
mod trace_logger;

#[derive(Debug, Clone)]
pub struct AlphaBetaInnerPaths{
    sum_alpha_paths: usize,
    sum_beta_paths: usize,
    sum_inner_paths: usize,
}

impl AlphaBetaInnerPaths {
    pub fn new( sum_alpha_paths: usize,
                sum_beta_paths: usize,
                sum_inner_paths: usize,) -> Self {
        Self {
            sum_alpha_paths,
            sum_beta_paths,
            sum_inner_paths
        }
    }

    fn fmt_log_entry(&self, f: &mut fmt::Formatter) -> FmtResult {
        let width = 100;

        writeln!(f, "\n{:-^w$}", " Tres Sections Paths MD ", w = width)?;

        writeln!(f, "\nThere exists {} paths between the Alpha node and Sierra (source node).", self.sum_alpha_paths)?;
        writeln!(f, "There exists {} paths between the Beta node and Tau (sink node).", self.sum_beta_paths)?;
        writeln!(f, "There exists {} (inner) paths between the Alpha node and Beta node.", self.sum_inner_paths)?;
        writeln!(f, "The trivial path should *not* be present, as these numbers are based on the candidates!")?;

        Ok(())
    }
}

pub enum DisplayAbiPaths<'a> {
    AsLog(&'a AlphaBetaInnerPaths)
}

impl fmt::Display for DisplayAbiPaths<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        use DisplayAbiPaths::*;

        match self {
            AsLog(abi) => abi.fmt_log_entry(f)
        }
    }
}