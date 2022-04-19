pub mod bthandler_trait;

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::io::{Error, ErrorKind};


pub const PROB_FACTOR: usize =  1000;

#[derive(Debug, Clone,)]
pub struct BaseTable {
    table: Vec<Vec<usize>>,
    prob_exponents: BTreeMap<usize, usize>,
    k: f64,
}

#[allow(dead_code)]
impl BaseTable {

    pub fn new(table: Vec<Vec<usize>>) -> Result<BaseTable, Error> {
        Self::try_from(table)
    }

    pub fn row(&self, row_nr: usize) -> Option<&Vec<usize>> {
        self.table.get(row_nr)
    }

    pub fn column(&self, col_nr: usize) -> Vec<usize> {
        (0..self.table.len())
            .map(|row| self.get_entry(row as u8, col_nr as u8)
                .expect("Entry not found: Index out of bounds"))
            .collect()
    }

    pub fn nr_of_rows(&self) -> usize {
        self.table.len()
    }

    pub fn nr_of_columns(&self) -> usize {
        self.table[0].len()
    }

    pub fn get_entry(&self, row: u8, column: u8) -> Option<usize> {
        self.table.get(row as usize)?.get(column as usize).cloned()
    }


    /// Returns 'k', where 'k' is the weighted average value of the probability exponents of this
    /// BaseTable instance.
    pub fn k(&self) -> f64 {
        self.k
    }

    /// Returns all the probability exponents present in this BaseTable,
    /// Obs, entry 0 will return infinity or an impossibly large number. (Infinity times PROB_FACTOR
    /// turns out to be a infinitely less than infinity, yet still quite too large for our purposes).
    pub fn prob_exponents(&self) -> &BTreeMap<usize, usize> {
        &self.prob_exponents
    }

    /// Returns the 'probability exponent' for the given entry, where an entry is the number of times
    /// the output difference/weight/etc occurs for the input difference/weight/etc.
    ///
    /// The 'probability exponent' for an entry in the BaseTable is the log<sub>2</sub> of the entry's contribution
    /// to the overall probability of the characteristic/bias(?).
    /// > Example:
    /// >    The probability of an entry is 2<sup>-2</sup>. Then the probability exponent is
    /// >    log<sub>2</sub> 2<sup>-2</sup> = -2.
    pub fn prob_exponent_for_entry(&self, entry: usize) -> Option<usize> {
        self.prob_exponents.get(&entry).cloned()
    }

    pub fn table_hashed(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.table.hash(&mut s);
        s.finish()
    }


    // ========================== Private Functions ======================================

    /// 'k' is what we've called the weighted average value of the probability exponent of this
    /// BaseTable instance. This fn calculates this value.
    fn calculate_k(table: &Vec<Vec<usize>>, probs: &BTreeMap<usize, usize>) -> f64 {

        let mut counts =  BTreeMap::new();
        // Skipping the 0 row and 0 column, as they should always be 0. (Except at 0,0).
        for row in 1..table.len() {
            for col in 1.. table[0].len() {
                let entry = table[row][col];
                let count = counts.entry(entry).or_insert(0);
                *count += 1;
            }
        }

        let tot_counts = counts.iter()
            .filter(|(key, _)| key != &&0)
            .fold(0, |acc, (_, val)| acc + val) as f64;

        let k = counts.iter()
            .filter(|(key, _)| key != &&0)
            .fold(0_f64, |acc, (key, val)| {
                acc + (*val as f64/tot_counts)*(*probs.get(key).unwrap() as f64 / PROB_FACTOR as f64)
            });

        // println!("Found k: {}", k);
        k

    }

    /// The probability exponent for an entry in the BaseTable is the log2 of the entry's contribution
    /// to the overall probability of the characteristic.
    /// Example:
    ///     The probability of an entry is 2^(-2). Then the probability exponent is
    ///     log2(2^(-2)) = -2.
    fn calculate_prob_exponents(table: Vec<Vec<usize>>) -> BTreeMap<usize, usize> {
        // 0,0 should always be present,
        let denom = table[0][0];

        let mut probs = BTreeMap::new();

        for row in 0..table.len(){
            for col in 0..table[0].len() {
                let entry = table[row][col];
                if !probs.contains_key(&entry) {
                    let e: f64 = entry as f64;
                    let raw = -(e / denom as f64).log2();
                    probs.insert(entry, (raw * PROB_FACTOR as f64) as usize );
                }
            }
        }
        probs
    }
}



impl TryFrom<Vec<Vec<usize>>> for BaseTable {
    type Error = std::io::Error;

    fn try_from(table: Vec<Vec<usize>>) -> Result<Self, Self::Error> {
        if table.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "The DDT cannot be empty"));
        }

        let nr_of_cols = table[0].len();
        if nr_of_cols == 0 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "The DDT cannot be empty: We have no columns"));
        }

        for row in table.iter() {
            if row.len() != nr_of_cols {
                return Err(Error::new(ErrorKind::InvalidInput,
                                      "The DDT cannot have varying number of columns"));
            }
        }

        let prob_exponents = Self::calculate_prob_exponents(table.clone());
        let k = Self::calculate_k(&table, &prob_exponents);

        Ok( BaseTable {
            table,
            prob_exponents,
            k,
        } )
    }
}