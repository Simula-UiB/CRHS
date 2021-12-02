use structopt::StructOpt;
use std::path::PathBuf;

#[derive(Clone, StructOpt)]
#[structopt(
name = "SoCCs",
about = "Differential and Linear hull search using SoCs."
)]
pub enum DlOptions {
    #[structopt(name = "differential")]
    Diff {
        #[structopt(short = "c", long = "cipher")]
        /// Name of the cipher to analyze.
        cipher: String,

        #[structopt(long = "soft_lim", required_unless("soft-lim-exponent"))]
        /// Maximum number of nodes the SoC may contain before starting the pruning operation,.
        soft_lim: Option<usize>,

        #[structopt(short = "e", long = "exponent", required_unless("soft-lim"))]
        /// Base2 log of maximum number of nodes the SoC may contain before starting the pruning operation.
        soft_lim_exponent: Option<usize>,

        #[structopt(short = "r", long = "rounds")]
        num_rounds: usize,


        #[structopt(short = "o", long = "out")]
        /// Folder to output generated SoC and other results.
        /// Filename will be deduced from cipher and meta
        out_parent_folder: PathBuf,

        #[structopt(short = "f", long = "folder")]
        /// Folder where the Solved Soc end meta is stored.
        /// Filename will be deduced from cipher and meta
        in_parent_folder: Option<PathBuf>,

        #[structopt(short = "s")]
        /// Will hide the end output if set. That is, the progress bars will still show,
        /// but not the end results.
        silent_mode: bool,
    },

    #[structopt(name = "linear")]
    Lin {
        #[structopt(short = "c", long = "cipher")]
        /// Name of the cipher to analyze.
        cipher: String,

        #[structopt(long = "soft_lim", required_unless("soft-lim-exponent"))]
        /// Maximum number of nodes the SoC may contain before starting the pruning operation,.
        soft_lim: Option<usize>,

        #[structopt(short = "e", long = "exponent", required_unless("soft-lim"))]
        /// Base2 log of maximum number of nodes the SoC may contain before starting the pruning operation.
        soft_lim_exponent: Option<usize>,

        #[structopt(short = "r", long = "rounds")]
        num_rounds: usize,

        #[structopt(short = "o", long = "out",)]
        /// Folder to output generated SoC and other results.
        /// Filename will be deduced from cipher and meta
        out_parent_folder: PathBuf,

        #[structopt(short = "f", long = "folder")]
        /// Folder where the Solved Soc end meta is stored.
        /// Filename will be deduced from cipher and meta
        in_parent_folder: Option<PathBuf>,

        #[structopt(short = "s")]
        /// Will hide the end output if set. That is, the progress bars will still show,
        /// but not the end results.
        silent_mode: bool,
    },

    #[structopt(name = "cg")]
    CG {
        #[structopt(short = "e", long = "exponent")]
        /// Base2 log of maximum number of nodes the SoC may contain before starting the pruning operation.
        soft_lim_exponent: Option<usize>,

        #[structopt(short = "o", long = "out")]
        /// Folder to output generated SoC and other results.
        /// Filename will be deduced from cipher and meta
        out_parent_folder: PathBuf,

        #[structopt(short = "f", long = "folder")]
        /// Folder where the Solved Soc end meta is stored.
        /// Filename will be deduced from cipher and meta
        in_parent_folder: Option<PathBuf>,

        #[structopt(short = "b")]
        /// The various ciphers are divided up into batches, to allow for parallel runs without
        /// messing anything up. Currently divided into 4 batches.
        /// Valid inputs are in the range from 0..batches.len() (i.e. 0..4).
        batch: u8,

        #[structopt(short = "l")]
        /// Will run in linear analysis mode if set.
        linear: bool,

        #[structopt(short = "d")]
        /// Will run in differential analysis mode if set.
        differential: bool
    },

}