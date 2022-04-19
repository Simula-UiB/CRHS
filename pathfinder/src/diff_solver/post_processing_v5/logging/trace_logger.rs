use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub use super::cache::{Cache, LogType};
use super::DisplayPreSessEstimateMD;
pub use super::PreSessEstimateMD;
use crate::diff_solver::post_processing_v5::sess_handling::DisplaySessEst;
use crate::diff_solver::post_processing_v5::DisplayResult;
use crate::diff_solver::post_processing_v5::logging::DisplayAbiPaths;


/// The Cache struct comes with the functionality of allowing to output the metadata to file as the
/// analysis progress, providing real time meta-data. However, the Cache itself does not handle writing
/// to file. Instead, it delegates to the user to supply it with a channel to a consumer. By doing so,
/// Cache allows for more advanced analysis to be performed on the metadata, if so desired by the end
/// user.
///
/// If the user is fine with simply tracing the metadata, by writing it to file as it is sent, then
/// we provide this simple TraceLogger struct to do precisely that: It will write the "packets", aka
/// LogTypes, to the supplied file as it receives them. This is especially useful when a panic is
/// caused for some unknown reason.
pub struct TraceLogger
{
    rx: Receiver<LogType>,
    out_file: PathBuf,
}

impl TraceLogger
{

    /// Constructs a new TraceLogger, and spawns it of into its own thread.
    /// 'out_file' is to be the file where the logger writes its output to. We will open this file
    /// with append = false and truncate = true.
    ///
    /// Any communication with the logger is done through the returned sender.
    ///
    /// Use this fn if all you want is a very basic logger, with no extra functionality or settings.
    pub fn init_and_run(out_file: PathBuf) -> Sender<LogType> {
        let (tx, rx) = channel();
        let logger = TraceLogger::new(out_file,
                                      false,
                                      rx)
            .expect("Failed to init logger.");

        thread::spawn(move || {
            logger.run()
        });
        tx
    }

    ///
    pub fn new(out_file: PathBuf,
               append: bool,
               rx: Receiver<LogType>) -> std::io::Result<TraceLogger> {

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .truncate(!append)
            .open(&out_file)?;
        writeln!(file, "{: ^100}", "Metadata as Trace")?;
        writeln!(file, " Metadata will be outputted here when logged, almost like a metadata trace.")?;
        writeln!(file, " It is intended to give insight \"under the hood\", so to speak, to whomever is interested.")?;
        writeln!(file, " Eventually, the plan is to make each section self explanatory.")?;

        Ok(Self {
            rx,
            out_file,
        })
    }


    pub fn run(self) {
        use LogType::*;
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(self.out_file)
            .expect("File not found, or couldn't open the file");

        for entry in self.rx {
            match &entry {
                PreSessEstimateMDEntry(pse) => {
                    writeln!(file, "{}", DisplayPreSessEstimateMD::AsLogEntry(pse)).unwrap();
                },
                SessEstimate(est) => { writeln!(file, "{}", DisplaySessEst::AsLog(est)).unwrap() },
                LayoutMaster(md) => {
                    writeln!(file, "{}", DisplayPreSessEstimateMD::MlAsLogEntry(&md)).unwrap();
                },
                ResultSection(section) => {
                    writeln!(file, "{}", DisplayResult::SectionAsSummary(section)).unwrap();
                },

                AlphaBetaInnerPaths(abi) => {
                    writeln!(file,"{}", DisplayAbiPaths::AsLog(abi)).unwrap();
                },
            };

        }
        // Some finishing comments?
    }
}