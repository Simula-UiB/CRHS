use std::collections::Bound;
use std::convert::TryFrom;
use std::fmt::{Alignment, Display, Formatter, Result as FmtResult};
use std::fmt::Error as FmtError;
use std::fmt::Write;
use std::iter::FromIterator;
use std::ops::{Range, RangeBounds};

use vob::{vob, Vob};

use crush::algebra::Matrix;

use crate::code_gen::SBoxHandler;
use crate::diff_solver::post_processing_v5::utils;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Path {
    path: Vob,
}


pub enum DisplayPath<'a> {
    /// (Path, block size, number of rounds)
    CompleteBin(&'a Path, usize, usize),
    /// (Path, block size, number of rounds)
    CompleteHex(&'a Path, usize, usize),
    /// (Path, print_header)
    OneLinerBin(&'a Path, bool),
    /// (Path, print_header)
    OneLinerHex(&'a Path, bool),
}

impl Path {
    pub fn append(&mut self, tail: &Path) {
        self.path.extend_from_vob(&tail.path.clone());
    }

    /// Expand self into a complete Path. Assumes LSB is at index 0.
    /// The path is "de-leaved", meaning that each 'block-size' segment of the path is either an
    /// input or an output from an S-box layer. THis is in contrast to a path 'fresh out of the Matrix op',
    /// which is interleaved, meaning that each 2*block-size length segment has input and output\
    /// from the same S-box layer interleaved. (Input and output from same S-box is adjacent).
    pub(crate) fn expand_to_full_path<S: SBoxHandler>(&self, lhss: &Matrix, sbh: &S, num_rounds: usize) -> Self {
        let mut b = Vob::with_capacity(self.path.len()*2);
        debug_assert_eq!(num_rounds%2, 0);

        // Perform a 'Mx = b' op, where lhss is the 'M', and self.path = 'x'. b is the 'b'
        for lhs in lhss.iter_rows() {
            let mut lhs = lhs.clone();
            lhs.and(&self.path);
            b.push(lhs.iter_set_bits(..).count() % 2 == 1);
        }

        // b is interleaved: Since the input ant output bits lies adjacent in LHSs, so does the
        // bits in b => We need to "de-leave" the various rounds.
        let b: Path = b.into();
        let mut res = Vob::with_capacity(b.len());

        let mut start_i;
        let mut end_i= 0;

        for r in 0..num_rounds {
            let mut inn = Vob::new();
            let mut out = Vob::new();
            for s in 0..sbh.num_sboxes(r) {
                start_i = end_i;
                end_i += sbh.sbox_size_in(r, s);
                let inn_p = &b.bits_in_range(Range{start: start_i, end: end_i});
                inn.extend_from_vob(&(inn_p).into());
                start_i = end_i;
                end_i += sbh.sbox_size_out(r, s);
                let out_p = &b.bits_in_range(Range{start: start_i, end: end_i});
                out.extend_from_vob(&out_p.into());

            }
            res.extend_from_vob(&inn);
            res.extend_from_vob(&out);

        }

        res.into()
    }


    // TODO benchmark: Is this faster, or would going through a Vec<bool> be faster?
    /// Returns the bits within the range given.
    pub fn bits_in_range<R>(&self, range: R) -> Path
        where
            R: RangeBounds<usize>,
    {
        let range = self.process_range_bounds(range);

        if self.path.iter_set_bits(range.start..range.end).next().is_none() {
            return vob![range.len(); false].into()
        }

        let mut ret = self.path.clone();
        ret.truncate(range.end);
        ret.split_off(range.start).into()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.path.len()
    }


    #[inline]
    fn process_range_bounds<R>(&self, bounds: R) -> Range<usize>
        where
            R: RangeBounds<usize>,
    {
        use Bound::{Included, Excluded, Unbounded};
        // start >= self.path.start ==> 0,
        // end <= self.path.end, depending on inclusive or exclusive
        // start <= end, depending on inclusive or exclusive
        let len = self.path.len();
        let start = match bounds.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            Included(&n) => n + 1,
            Excluded(&n) => n,
            Unbounded => len,
        };

        // Invariant checks
        if start > end {
            panic!("Start index cannot be after the end index: Start {}, end {}", start, end);
        }
        if end > len {
            panic!("Index out of bounds. End: {}, len: {}", end, len);
        }

        Range{start, end}
    }

}

// ================================================================================================
// ==================================== Trait Impls ===============================================
// ================================================================================================

impl TryFrom<&Path> for u128 {
    type Error = String;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if path.len() > 128 {
            return Err("Path is too long: It exceeds 128 edges".to_owned());
        }
        let bools: Vec<bool> = path.path.iter().collect();
        let val = bools.iter().enumerate().take(128)
            .fold(0u128, |acc, (idx, x)| { acc | ((*x as u128) << idx)});

        Ok(val)
    }

}

impl From<u128> for Path {
    fn from(int: u128) -> Self {
        let as_bytes = int.to_be_bytes();
        let vob = Vob::from_bytes(&as_bytes);
        let vob: Vob = vob.iter().rev().collect();

        Path {
            path: vob,
        }
    }
}

impl From<&Path> for Vec<bool> {
    fn from(path: &Path) -> Self {
        path.path.iter().map(|v| v.clone()).collect()
    }
}

impl From<Vec<bool>> for Path {
    fn from(path: Vec<bool>) -> Self {
        Path {
            path: Vob::from_iter(path),
        }
    }
}

impl From<Vob> for Path {
    fn from(path: Vob<usize>) -> Self {
        Path {
            path,
        }
    }
}

impl From<&Vob> for Path {
    fn from(path: &Vob<usize>) -> Self {
        Path {
            path: path.clone(),
        }
    }
}

impl From<&mut Vob> for Path {
    fn from(path: &mut Vob<usize>) -> Self {
        Path {
            path: path.clone(),
        }
    }
}

impl From<&Path> for Vob {
    fn from(path: &Path) -> Self {
        let mut v = Vob::with_capacity(path.len());
        v.extend_from_vob(&path.path);
        v
    }
}

impl From<Path> for Vob {
    fn from(path: Path) -> Self {
        let mut v = Vob::with_capacity(path.len());
        v.extend_from_vob(&path.path);
        v
    }
}

// ================================================================================================
// ====================================== PathDisplayMode =========================================
// ================================================================================================

impl Display for DisplayPath<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {

        match self {
            DisplayPath::CompleteBin(path, block_size, rounds) => {
                Self::fmt_complete_path(&(*path), f, *block_size, *rounds, PrintMode::Bin)?;
            }
            DisplayPath::CompleteHex(path, block_size, rounds) => {
                Self::fmt_complete_path(&(*path), f, *block_size, *rounds, PrintMode::Hex)?;
            },
            DisplayPath::OneLinerBin(path, print_header) => {
                Self::fmt_one_liner(&*path, f, *print_header, PrintMode::Bin)?;
                 },
            DisplayPath::OneLinerHex(path, print_header) => {
                Self::fmt_one_liner(&*path, f, *print_header, PrintMode::Hex)?;
            },
        }

        Ok(())
    }
}

enum PrintMode {
    Bin,
    Hex,
}

impl DisplayPath<'_> {

    /// Print fn which prints a complete path (AKA "expanded path"), meaning a path which has all
    /// input and output states to all non-linear layers, and ordered accordingly. It will printout
    /// the various states on separate lines, starting with alpha, and put "state separators"
    /// between the states.
    /// A "state separator" is either the string "\"S-box layer\"" or "Linear layer", essentially
    /// giving the reader context to understand where each state belongs in the bigger picture.
    /// In addition, each new round will be marked as such, and alpha and beta will also be marked
    /// as alpha and beta, respectively.
    ///
    /// A header indicating where MSB and LSB are located will also be printed.
    ///
    /// *It us up to the caller* to ensure that the given path is indeed an expanded path, and that the
    /// path corresponds to the given parameters 'block_size' and 'num_rounds': *The formatting will
    /// otherwise be off.*
    ///
    /// The fn may print in Hex or Bin, depending on what PrintMode it is given.
    /// In addition, it will also respect any width, align and fill parameters given through
    /// the fmt::Formatter.
    fn fmt_complete_path(path: &Path, f: &mut Formatter<'_>, block_size: usize, num_rounds: usize,
                         mode: PrintMode )
                        -> FmtResult
    {
        use PrintMode::{Bin, Hex};
        // Setting up params
        let state_width = match mode {
            Bin => block_size,
            // Divide by 4 to get the state width when hex
            Hex => block_size/4,
        };
        // Default to state_width, allows for safe subtraction later
        let width = match f.width() {
            None => state_width,
            Some(w) => w.max(state_width),
        };
        let fill = f.fill();

        let align = match f.align() {
            Some(align) => align,
            // Default to Left alignment
            None => Alignment::Left,
        };

        // Make header
        let mut header = String::new();
        let post_pad = Self::padding(&mut header, width - state_width, fill, &align)?;
        write!(header, "MSB{: ^w$}LSB", "", w = state_width.checked_sub(6).unwrap_or(7))?;
        Self::post_pad(&mut header, post_pad, fill)?;
        writeln!(f, "{}", header)?;

        // Write the rest
        let path: Vec<bool> = path.into();
        for (i, round) in path.chunks(block_size).enumerate() {
            debug_assert_eq!(round.len(), block_size);

            let r = match mode {
                Bin => utils::bools_to_bin_string(round)?,
                Hex => utils::bools_to_hex_string(round)?,
            };

            let mut buff = String::new();
            let post_pad = Self::padding(&mut buff, width - state_width, fill, &align)?;
            write!(buff, "{}", r)?;

            // First state == Alpha path
            if i == 0 {
                write!(buff, ": Alpha")?;
                Self::post_pad(&mut buff, post_pad.checked_sub(7).unwrap_or(0), fill)?;

                // Last state == Beta path
            } else if i == num_rounds*2 -1 {
                write!(buff, ": Beta")?;
                Self::post_pad(&mut buff, post_pad.checked_sub(6).unwrap_or(0), fill)?;
                write!(f, "{}", buff)?; // Write to f before we break! However, do not force a linebreak on the caller
                return Ok(());

                // "Internal" states/paths. I.e. not alpha nor beta
            } else {
                // First case: State is the first state in a new round => append postfix
                if i % 2 == 1 {
                    write!(buff, ": New round")?;
                    Self::post_pad(&mut buff, post_pad.checked_sub(11).unwrap_or(0), fill)?;
                } else {
                    //Second case: No prefix to be added, post-pad as normal
                    Self::post_pad(&mut buff, post_pad, fill)?;
                }

            }

            // "Flush" state to f before writing the "state separators"
            writeln!(f, "{}", buff)?;
            buff.truncate(0);
            let post_pad = Self::padding(&mut buff, width - state_width, fill, &align)?;
            // Write state "separators"
            if i%2 == 0 {
                write!(buff, "{: ^w$}", "\"S-box Layer\"", w = state_width)?;
            } else {
                write!(buff, "{: ^w$}", "Linear Layer", w = state_width)?;
            }
            Self::post_pad(&mut buff, post_pad, fill)?;
            // Flush buff to f
            writeln!(f, "{}", buff)?;
        }

        Ok(())
    }

    /// Print fn intended for to print single states, such as alpha and beta. The print allows for
    /// an optional header to be printed on the line above, indicating where the LSB and MSB are
    /// located.
    ///
    /// The fn may print in Hex or Bin, depending on what PrintMode it is given.
    ///
    /// Even though this fn is intended for a single state, it is not restrained to only printing
    /// such. It will print whatever vec of bools given, on a single line.
    /// It will also respect any width, align and fill parameters given through the fmt::Formatter.
    fn fmt_one_liner(path: &Path, f: &mut Formatter<'_>, print_header: bool, mode: PrintMode)
                     -> FmtResult
    {
        use PrintMode::{Bin, Hex};
        // Setting up params
        let state_width = match mode {
            Bin => path.len(),
            // Divide by 4 to get the state width when hex
            Hex => path.len()/4,
        };
        // Default to state_width, allows for safe subtraction later
        let width = match f.width() {
            None => state_width,
            Some(w) => w.max(state_width),
        };
        let fill = f.fill();

        let align = match f.align() {
            Some(align) => align,
            // Default to Left alignment
            None => Alignment::Left,
        };

        if print_header {
            let mut header = String::new();
            let post_pad = Self::padding(&mut header, width - state_width, fill, &align)?;
            write!(header, "MSB{: ^w$}LSB", "", w = state_width.checked_sub(6).unwrap_or(7))?;
            Self::post_pad(&mut header, post_pad, fill)?;
            writeln!(f, "{}", header)?;
        }

        let path: Vec<bool> = path.into();
        let r = match mode {
            Bin => utils::bools_to_bin_string(&path)?,
            Hex => utils::bools_to_hex_string(&path)?,
        };
        let mut buff = String::new();
        let post_pad = Self::padding(&mut buff, width - state_width, fill, &align)?;
        write!(buff, "{}", r)?;
        Self::post_pad(&mut buff, post_pad.checked_sub(6).unwrap_or(0), fill)?;
        write!(f, "{}", buff)?; // Write!, not writeln! As we do not to force a linebreak on the caller

        Ok(())
    }

    /// Write the pre-padding and return the length of the unwritten post-padding. Callers are
    /// responsible for ensuring post-padding is written after the thing that is being padded.
    fn padding(buff: &mut String, padding: usize, fill: char, align: &Alignment)
        -> Result<usize, FmtError>
    {
        // Kudos to the author of fmt::padding(), whom this solution is modelled after.

        let (pre_pad, post_pad) = match align {
            Alignment::Left => (0, padding),
            Alignment::Center => (padding / 2, (padding + 1) / 2),
            Alignment::Right => (padding, 0),
        };

        for _ in 0..pre_pad {
            buff.write_char(fill)?;
        }

        Ok(post_pad)
    }

    /// Write the post-padding.
    fn post_pad(buff: &mut String, post_pad: usize, fill: char) -> FmtResult {
        for _ in 0..post_pad {
            buff.write_char(fill)?;
        }
        Ok(())
    }
}


// ================================================================================================
// ========================================= Tests ================================================
// ================================================================================================

#[cfg(test)]
mod tests {
    use crush::soc::utils::{build_system_from_spec, parse_system_spec_from_file};

    use crate::ciphers::prince::SbMock;

    use super::*;

    #[test]
    fn test_display_complete_path_bin() {
        // LSB is index 0
        let alpha: Vec<bool> = "0000000000000000000000000000000000001000000010000000000000000000"
            .chars()
            .map(|c| {
                if c == '0' { return false }
                if c == '1' { return true }
                panic!("Invalid char encountered: {}", c);
            })
            // .rev()
            .collect();
        assert_eq!(alpha.len(), 64);
        assert_eq!(alpha[36], true, "36");
        assert_eq!(alpha[44], true, "44");

        let expected = "0000000000000000000100000001000000000000000000000000000000000000: Alpha";
        let header_hex = format!("{}", DisplayPath::CompleteBin(&alpha.into(), 64, 4));
        let actual = header_hex.split("\n").skip(1).next().unwrap();
        assert_eq!(actual, expected);

        let eight_rounds_raw: Vec<bool> =
            "0000000000000000000000000000000000001000000010000000000000000000\
            0000000000000000000000000000000000000001000000010000000000000000\
            0000000000000000000100000000000000000000000000000000000000010000\
            0000000000000000001000000000000000000000000000000000000000010000\
            0000000000000000001000100000001000000000000000000000000100010001\
            0000000000000000100000010000001000000000000000000000001010000001\
            0000000000000000000000100000001010000000100000000000000100000001\
            0000000000000000000000010000000110000000100000000000100000001000"
                .chars()
                .map(|c| {
                    if c == '0' { return false }
                    if c == '1' { return true }
                    panic!("Invalid char encountered: {}", c);
                })
                .collect();

        let expected_states = vec![
            "0000000000000000000100000001000000000000000000000000000000000000: Alpha",
            "0000000000000000100000001000000000000000000000000000000000000000: New round",
            "0000100000000000000000000000000000000000000010000000000000000000",
            "0000100000000000000000000000000000000000000001000000000000000000: New round",
            "1000100010000000000000000000000001000000010001000000000000000000",
            "1000000101000000000000000000000001000000100000010000000000000000: New round",
            "1000000010000000000000010000000101000000010000000000000000000000",
            "0001000000010000000000010000000110000000100000000000000000000000: Beta"
        ];

        println!("{: ^150}", DisplayPath::CompleteBin(&eight_rounds_raw.clone().into(), 64, 4));

        let actual_full = format!("{}", DisplayPath::CompleteBin(&eight_rounds_raw.into(), 64, 4));
        let actual_lines = actual_full.split("\n");
        let actual_lines: Vec<&str> = actual_lines.into_iter().collect();
        assert_eq!(actual_lines[1], expected_states[0], "0");
        assert_eq!(actual_lines[3], expected_states[1], "1");
        assert_eq!(actual_lines[5], expected_states[2], "2");
        assert_eq!(actual_lines[7], expected_states[3], "3");
        assert_eq!(actual_lines[9], expected_states[4], "4");
        assert_eq!(actual_lines[11], expected_states[5], "5");
        assert_eq!(actual_lines[13], expected_states[6], "6");
        assert_eq!(actual_lines[15], expected_states[7], "7");
    }

    #[test]
    fn testing() {
        // LSB is index 0
        let alpha: Vec<bool> = "0000000000000000000000000000000000001000000010000000000000000000"
            .chars()
            .map(|c| {
                if c == '0' { return false }
                if c == '1' { return true }
                panic!("Invalid char encountered: {}", c);
            })
            // .rev()
            .collect();
        assert_eq!(alpha.len(), 64);
        assert_eq!(alpha[36], true, "36");
        assert_eq!(alpha[44], true, "44");

        let expected = "0000101000000000: Alpha";
        let header_hex = format!("{}", DisplayPath::CompleteHex(&alpha.into(), 64, 4));
        let actual = header_hex.split("\n").skip(1).next().unwrap();
        assert_eq!(actual, expected);

        let eight_rounds_raw: Vec<bool> =
           "0000000000000000000000000000000000001000000010000000000000000000\
            0000000000000000000000000000000000000001000000010000000000000000\
            0000000000000000000100000000000000000000000000000000000000010000\
            0000000000000000001000000000000000000000000000000000000000010000\
            0000000000000000001000100000001000000000000000000000000100010001\
            0000000000000000100000010000001000000000000000000000001010000001\
            0000000000000000000000100000001010000000100000000000000100000001\
            0000000000000000000000010000000110000000100000000000100000001000"
                .chars()
                .map(|c| {
                    if c == '0' { return false }
                    if c == '1' { return true }
                    panic!("Invalid char encountered: {}", c);
                })
                .collect();

        let expected_states = vec![
            "0000101000000000: Alpha",
            "0000808000000000: New round",
            "0800000000080000",
            "0800000000040000: New round",
            "8880000040440000",
            "8140000040810000: New round",
            "8080010140400000",
            "1010010180800000: Beta"
        ];

        println!("{: ^150}", DisplayPath::CompleteHex(&eight_rounds_raw.clone().into(), 64, 4));

        let actual_full = format!("{}", DisplayPath::CompleteHex(&eight_rounds_raw.into(), 64, 4));
        let actual_lines = actual_full.split("\n");
        let actual_lines: Vec<&str> = actual_lines.into_iter().collect();
        assert_eq!(actual_lines[1], expected_states[0], "0");
        assert_eq!(actual_lines[3], expected_states[1], "1");
        assert_eq!(actual_lines[5], expected_states[2], "2");
        assert_eq!(actual_lines[7], expected_states[3], "3");
        assert_eq!(actual_lines[9], expected_states[4], "4");
        assert_eq!(actual_lines[11], expected_states[5], "5");
        assert_eq!(actual_lines[13], expected_states[6], "6");
        assert_eq!(actual_lines[15], expected_states[7], "7");
    }

    #[test]
    fn test_from_u128() {
        let mut v = Vob::from_elem(128, false);
        v.set(127, true);
        let expected = Path { path: v };
        let actual = Path::from(1_u128 << 127);

        assert_eq!(actual, expected);
        println!("Checkpoint 1");

        let mut v = Vob::from_elem(128, false);
        v.set(127, true);
        v.set(125, true);
        let expected = Path { path: v,};
        let actual = Path::from(1_u128 << 127 | 1_u128 << 125);

        assert_eq!(actual, expected);
        println!("Checkpoint 2");

        let mut v = Vob::from_elem(128, false);
        v.set(1, true);
        let expected = Path { path: v,};
        let actual = Path::from(2);

        assert_eq!(actual, expected);
        println!("Checkpoint 3");

        let mut v = Vob::from_elem(128, false);
        v.set(1, true);
        v.set(2, true);
        let expected = Path { path: v};
        let actual = Path::from(6);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_into_u128() -> Result<(), String> {
        let mut v = Vob::from_elem(128, false);
        v.set(127, true);
        let path = Path { path: v};

        let actual = u128::try_from(&path)?;
        assert_eq!(actual, 1 << 127);
        println!("Checkpoint 1");

        let mut v = Vob::from_elem(128, false);
        v.set(1, true);
        let path = Path { path: v};

        let actual = u128::try_from(&path)?;
        assert_eq!(actual, 2);
        println!("Checkpoint 2");

        Ok(())
    }


    #[ignore]
    #[test]
    fn test_path_expansion() -> FmtResult {
        let bin = &[
            0b00000000, 0b00000000, 0b00000001, 0b00000001, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000010, 0b00000010, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00000000, 0b00000000, 0b00000001, 0b00000000,
            0b00100000, 0b00100000, 0b00000000, 0b00000000, 0b00010000, 0b00010000, 0b00000010, 0b00000010,
            0b00000010, 0b00000001, 0b00000000, 0b00000000, 0b00000001, 0b00000010, 0b00010000, 0b00100000,
            0b00010000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00010000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00001000, 0b00001000, 0b00000000, 0b00000000, 0b00000000, 0b00000000];
        let path = Path {
            path: Vob::from_bytes(bin)
        };

        let mut buff = String::new();

        writeln!(buff, "Path pre expansion")?;
        writeln!(buff, "{}", DisplayPath::CompleteBin(&path, 64, 6))?;

        let actual = path.expand_to_full_path(&prince2_lhss(), &SbMock::new(), 6);
        writeln!(buff, "\nPath post expansion:")?;
        writeln!(buff, "{}", DisplayPath::CompleteBin(&actual, 64, 6))?;
        writeln!(buff, "\nPath post expansion (as hex):")?;
        writeln!(buff, "{}", DisplayPath::CompleteHex(&actual, 64, 6))?;

        println!("{}", buff);
        panic!("Forced panic! We still need 'expected' to compare to!");
    }

    #[test]
    fn test_range_extractor() -> Result<(), String> {
        // block size
        let bz = 64;
        let bin = &[
            0b00000001, 0b00000000, 0b00000000, 0b00000001, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000010, 0b00000010, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00000000, 0b00000000, 0b00000001, 0b00000000,
            0b00100000, 0b00100000, 0b00000000, 0b00000000, 0b00010000, 0b00010000, 0b00000010, 0b00000010,
            0b00000010, 0b00000001, 0b00000000, 0b00000000, 0b00000001, 0b00000010, 0b00010000, 0b00100000,
            0b00010000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00010000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00001000, 0b00001000, 0b00000000, 0b00000000, 0b00000000, 0b00000000];
        let path = Path {
            path: Vob::from_bytes(bin)
        };

        let actual = path.bits_in_range(Range{start: bz*0, end: bz*1});
        let expected = Path{ path: Vob::from_bytes(&[
            0b00000001, 0b00000000, 0b00000000, 0b00000001, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
        ])};
        println!("{:0>64b}", u128::try_from(&actual)?);
        println!("{:0>64b}", u128::try_from(&expected)?);
        assert_eq!(actual, expected);
        println!("Checkpoint 1");

        let actual = path.bits_in_range(Range{start: bz*1, end: bz*2});
        let expected = Path{ path: Vob::from_bytes(&[
            0b00000000, 0b00000000, 0b00000010, 0b00000010, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
        ])};
        println!("{:0>64b}", u128::try_from(&actual)?);
        println!("{:0>64b}", u128::try_from(&expected)?);
        assert_eq!(actual, expected);
        println!("Checkpoint 2");

        let actual = path.bits_in_range(Range{start: bz*2, end: bz*3});
        let expected = Path{ path: Vob::from_bytes(&[
            0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00000000, 0b00000000, 0b00000001, 0b00000000,
        ])};
        println!("{:0>64b}", u128::try_from(&actual)?);
        println!("{:0>64b}", u128::try_from(&expected)?);
        assert_eq!(actual, expected);
        println!("Checkpoint 3");

        let actual = path.bits_in_range(Range{start: bz*6, end: bz*7});
        let expected = Path{ path: Vob::from_bytes(&[
            0b00000000, 0b00000000, 0b00001000, 0b00001000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
        ])};
        println!("{:0>64b}", u128::try_from(&actual)?);
        println!("{:0>64b}", u128::try_from(&expected)?);
        assert_eq!(actual, expected);
        println!("Checkpoint 4");

        Ok(())
    }

    fn prince2_lhss() -> Matrix {
        let sys_spec = parse_system_spec_from_file(
            // Prince2 soft lim 20:
            &["SoCs", "PRINCE_2.bdd"].iter().collect());
        let soc_original = build_system_from_spec(sys_spec);

        let mut lhss = soc_original.get_system_lhs();
        lhss.sort_unstable_by(|a,b| a.0.cmp(&b.0));
        let lhss: Matrix = Matrix::from_rows(lhss.iter()
            .flat_map(|(_, lhs)| lhs.clone()).collect());

        lhss
    }
}

