
//! FIXME update and improve.
//! As it logs research data on ongoing research and development, some data will turn out to be
//! irrelevant and needs to be removed, while other is missed and needs to be added to the logging.
//! It is important that any of those changes are as easy to do as possible.
//! Also, there are usually needs for different outputs, or differently structured output, depending
//! on where in the process one is: F.ex. a live trace of a run-through vs after-running analysis.
//!
//! I'm still experimenting with how logging and tracing best can be executed. My goal is to make
//! something that is structured, flexible, able to output a live trace and also a summary at the
//! end.
//!
//! As not all data which logically belongs together are available at the same time, I've chosen to
//! divide the logging into two: 'Builder's and 'Record's.
//!
//! The 'Builder's deals with the live recording of data. They allow for data to be missing, and
//! they deal with any live output.
//!
//! The 'Record's are more of the long term records. All data which logically belong together are
//! expected to be available at the time of a records creation. (Unless it is intentionally meant
//! to be optional). This means that further/deeper analysis of the data may be possible than during
//! a ("live") run-through.

use records::PruneRecord;

pub mod builders;
pub mod records;

/// Quickfix
pub trait PruneLogger {
    fn record(&mut self, rec: PruneRecord);
}

