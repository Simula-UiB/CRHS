
use std::fmt;
use std::fmt::Result as FmtResult;

use console::style;

use crush::soc::bdd::differential::wd::{NcWDistribution};

use crate::diff_solver::post_processing_v5::sess_handling::SessEstimate;
use crate::diff_solver::post_processing_v5::utils::path::{DisplayPath, Path};
use std::collections::BTreeMap;
use super::BuildMode;
use super::ResultSectionBuilder;



// =================================================================================================
// =================================================================================================
// =================================================================================================
// =================================================================================================

pub struct ProcessedResult {
    sections: Vec<ProcessedResultSection>,
}

impl ProcessedResult {

    pub(super) fn new(sections: Vec<ResultSectionBuilder> ) -> ProcessedResult {
        let sections = sections.into_iter()
            .map(|s| s.into())
            .collect();

        Self {
            sections,
        }
    }

    fn fmt_summary(&self, f: &mut fmt::Formatter) -> FmtResult {
        for section in self.sections.iter() {
            section.fmt_as_summary(f)?;
        }
        Ok(())
    }
}


// =================================================================================================
// =================================================================================================
// =================================================================================================
// =================================================================================================


#[derive(Debug, Clone)]
pub struct ProcessedResultSection {
    mode: BuildMode,
    /// The actual SESS Estimate, with metadata
    best_estimate: SessEstimate,
    /// Number of paths in the hull
    paths_skipped: usize,
    /// The alpha path used
    alpha_path: Path,
    /// The beta path used
    beta_path: Path,
    /// An example path which will yield a best result. Should at least be a path which yields the
    /// fewest number of active S-boxes, but ideally one which also yields the lowest probability
    example_path: Path,
    /// The calculated probability for this hull
    hull_probability: f64,
    /// The various probability each path in the hull have, and how many paths have that probability
    /// OBS Keys are the actual probability/weight multiplied with a factor as given by the base table BT!
    probabilities_count: BTreeMap<usize, usize>,
    block_size: usize,
    num_rounds: usize,
}

impl ProcessedResultSection {

    fn fmt_as_summary(&self, buff: &mut fmt::Formatter) -> FmtResult {
        // Header
        writeln!(buff, "\n{:=^150}", style(" Differential/Hull Approximation Result: ").green())?;
        writeln!(buff, "{:=^150}\n", format!(" Mode: {} ", self.mode))?;

        // Approximate probability found
        writeln!(buff, "{: ^150}", format!("Approximate probability/bias of differential/hull is 2^(-{})\n",
                                           self.hull_probability))?;
        writeln!(buff, "{: ^150}\n", "-".repeat(10))?;
        writeln!(buff, "OBS, example path is part of the hull, but not necessarily the \"best\" path. To be fixed!")?;
        writeln!(buff, "{: ^150}\n", "-".repeat(10))?;

        // Alpha
        writeln!(buff, "{: ^150}", "Best *input* difference/mask found:")?;
        writeln!(buff, "{: ^150}", DisplayPath::OneLinerHex(&self.alpha_path, true))?;

        // Beta
        writeln!(buff, "{: ^150}", "Best *output* difference/mask found:")?;
        writeln!(buff, "{: ^150}\n", DisplayPath::OneLinerHex(&self.beta_path, true))?;

        writeln!(buff, "{: ^150}\n", "-".repeat(10))?;

        // Number of trails used to calculate differential/hull
        let tot = self.best_estimate.hull_distribution().unwrap().total_number_of_paths_overflowing();
        if tot.1 {
            writeln!(buff, "{: ^150}",
                     format!("The total number of paths overflowed a usize. The wrapped value is {}.",
                             tot.0))?;
        } else {
            writeln!(buff, "{: ^150}",
                     format!("We found {} trail(s), based on the in-out differences/masks from above,",
                             tot.0))?;
        }

        let skipped = self.paths_skipped;
        writeln!(buff, "{: ^150}",
                 format!( "of which {} paths were used and {} paths were skipped.\n",
            match skipped {
                0 => "all".to_string(),
                n => { if tot.1 {
                    format!("an unknown number of")
                } else {
                    format!("{}", tot.0 - n)
                }},
            },
            match skipped {
                0 => "no".to_string(),
                n => n.to_string(),
            }
        ))?;

        writeln!(buff, "{: ^150}\n", "-".repeat(10))?;

        // Example trail
        writeln!(buff, "{: ^150}", "Example trail (in hex):")?;
        writeln!(buff, "{: ^150}\n", DisplayPath::CompleteHex(&self.example_path,
                                                              self.block_size,
                                                              self.num_rounds))?;

        writeln!(buff, "{: ^150}\n", "-".repeat(10))?;

        writeln!(buff, "{: ^150}", "Example trail (as binary):")?;
        writeln!(buff, "{: ^150}\n", DisplayPath::CompleteBin(&self.example_path,
                                                            self.block_size,
                                                            self.num_rounds))?;

        writeln!(buff, "{: ^150}\n", "-".repeat(10))?;

        // Path weight distribution
        writeln!(buff,"{: ^150}",
                 format!("Observed path probability | Number of times"))?;
        for (w, c) in self.probabilities_count.iter() {
            writeln!(buff, "{: ^150}", format!("{: >25} | {: >3}{: >10}", w, c, ""))?;
        }

        Ok(())
    }
}

impl From<ResultSectionBuilder> for ProcessedResultSection {
    /// Call only on complete complete sections. Will otherwise panic on calls to unwrap.
    fn from(builder: ResultSectionBuilder) -> Self {
        Self {
            mode: builder.mode,
            best_estimate: builder.best_estimate,
            paths_skipped: builder.paths_skipped.unwrap(),
            alpha_path: builder.alpha_path.unwrap(),
            beta_path: builder.beta_path.unwrap(),
            example_path: builder.example_path.unwrap(),
            hull_probability: builder.hull_probability.unwrap(),
            probabilities_count: builder.probabilities_count.unwrap(),
            block_size: builder.block_size,
            num_rounds: builder.num_rounds
        }
    }
}

// =================================================================================================
// =================================================================================================
// =================================================================================================
// =================================================================================================


/// The official way of formatting a ProcessedResult for display to various outputs.
/// Text and formatting will vary depending on the "mode" requested.
pub enum DisplayResult<'a> {
    AsSummary(&'a ProcessedResult),
    SectionAsSummary(&'a ProcessedResultSection),
}

impl fmt::Display for DisplayResult<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> FmtResult {
        use DisplayResult::*;

        match self {
            AsSummary(report) => report.fmt_summary(f),
            SectionAsSummary(section) => section.fmt_as_summary(f),
        }
    }
}


// =================================================================================================
// =================================================================================================
// =================================================================================================
// =================================================================================================
