use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use vob::Vob;

use crush::algebra;
use crush::algebra::Matrix;
use crush::soc::bdd::differential::StyledProgressBar;
use crush::soc::Id;
use crush::soc::system::System;
use pathfinder::code_gen::{LLHandler, SBoxHandler};
use pathfinder::code_gen::gsf::GenericShard;
use pathfinder::code_gen::soc_gen;
use pathfinder::diff_solver::post_processing_v5::BaseTable;
use pathfinder::diff_solver::post_processing_v5::BTHandler;

use crate::dl::{DLmode, RawSoc, SolvedSoC, Loggers, Setup};
use crate::dl::cg_original::cipher::{Cipher, CipherStructure};
use crate::dl::progress::{MyStyledSpinner, Progress};

type RawTable = Vec<Vec<usize>>;
type Round = Vec<usize>;
type TableHash = u64;

// mod for dealing with the ciphers from CryptaGraph


pub struct CgBuilder ();

impl CgBuilder {

    /// Construct the SoC of the given cipher. The given cipher is used as a cipher spec to construct
    /// the associated SoC and other relevant metadata. The results are packed together and returned
    /// as a RawSoc.
    ///
    pub fn from_cipher(setup: &Setup, cipher: &dyn Cipher)
                       -> RawSoc<BtHandler, SbHandler>
    {
        match setup.cipher_structure {
            CipherStructure::Spn => {
                Self::spn(setup, cipher)
            },
            CipherStructure::Feistel => {
                panic!("Unsupported CipherStructure. Feistels are unfortunately not supported (yet?)")
            },
            CipherStructure::Prince => {
                Self::reflective(setup, cipher)
            },
        }
    }

    pub fn from_parent_folder(setup: &Setup,
                              cipher: &dyn Cipher,
                              progress_spinner: MyStyledSpinner,
                              in_parent_folder: PathBuf)
                              -> SolvedSoC<BtHandler, SbHandler, Progress>
    {
        progress_spinner.set_message("Building metadata.");
        let raw_soc = match setup.cipher_structure {
            CipherStructure::Spn => {
                Self::spn(setup, cipher)
            },
            CipherStructure::Feistel => {
                panic!("Unsupported CipherStructure. Feistels are unfortunately not supported (yet?)")
            },
            CipherStructure::Prince => {
                Self::reflective(setup, cipher)
            },
        };

        // Build file path
        let mut file_path = in_parent_folder;
        file_path.push(RawSoc::<BtHandler, SbHandler>::make_file_name(setup, &setup.cipher_name));
        file_path.set_extension("bdd");

        // Load SolvedSoc from file
        progress_spinner.println(&format!("Soc loaded from file: {}", file_path.display()));
        progress_spinner.set_message(&format!("Loading SoC from file: {}", file_path.display()));
        let sys_spec = crush::soc::utils::parse_system_spec_from_file(&file_path);
        let solved_soc = crush::soc::utils::build_system_from_spec(sys_spec);

        // assumes all out bits are equal! (We don't support unequal step anyways).
        let step = raw_soc.sb_handler.sbox_size_out(0,0);

        let active_area = Range{start: raw_soc.ll_handler.block_size(0),
            end: solved_soc.get_nvar() };

        progress_spinner.finish_with_message("Successfully loaded the Solved SoC from file.");
        SolvedSoC {
            setup: setup.clone(),
            soc: solved_soc,
            lhss: raw_soc.lhss,
            cohorts: raw_soc.cohorts,
            bt_handler: raw_soc.bt_handler,
            sb_handler: raw_soc.sb_handler,
            ll_handler: raw_soc.ll_handler,
            active_area,
            step: NonZeroUsize::new(step).unwrap(),
            loggs: Loggers::new(),
        }
    }

    fn reflective(setup: &Setup, cipher: &dyn Cipher) -> RawSoc<BtHandler, SbHandler>
    {
        // CipherStructure should be checked elsewhere
        assert_eq!(0, setup.num_rounds() % 2);

        let llh =  Self::reflective_llh(cipher, setup.num_rounds());
        let (bth, sbh) = Self::make_bth_sbh(cipher,
                                            setup.num_rounds(), setup.dl_mode());

        let soc = soc_gen::make_soc(&llh, &sbh, setup.num_rounds());

        Self::make_rawsoc(soc, Box::new(llh), bth, sbh, setup.clone())
    }


    fn spn(setup: &Setup, cipher: &dyn Cipher) -> RawSoc<BtHandler, SbHandler> {
        // This builder only builds for SPN like ciphers
        if cipher.structure() != CipherStructure::Spn {
            // Done this way, as CipherStructure does not implement Debug
            panic!("Invalid argument, cipher is of the wrong CipherStructure.");
        }

        let llh = Self::spn_llh(cipher);
        let (bth, sbh) = Self::make_bth_sbh(cipher,
                                            setup.num_rounds(), setup.dl_mode());

        let soc = soc_gen::make_soc(&llh, &sbh, setup.num_rounds());

        Self::make_rawsoc(soc, Box::new(llh), bth, sbh, setup.clone())
    }


    fn make_rawsoc((soc, rounds): (System, Vec<Vec<Id>>),
                   llh: Box<dyn LLHandler>,
                   bth: BtHandler,
                   sbh: SbHandler,
                   setup: Setup,
    )
                   -> RawSoc<BtHandler, SbHandler>
    {
        // Due to the way we construct our SoC's, we know that we're interested in the output bits
        // of any Shard. We therefore collect them together into cohorts, as requested by the
        // SimpleSolver.
        let insize = sbh.sbox_size_in[0][0];
        let cohorts = soc.iter_bdds()
            .map(|(id, shard)| {
                let to_keep = shard.borrow().get_lhs().iter().skip(insize)
                    .cloned()
                    .collect();
                (id.clone(), to_keep)
            }).collect();

        // Constructing the Matrix of all left-hand side linear combinations, as this is easiest to
        // do before we start any resolving of linear dependencies.
        let mut lhss = soc.get_system_lhs();
        // Sort Shards by id: We know that the construction process of the SoC will name the
        // Shards in inclining order, starting at Id::new(0).
        lhss.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let lhss: Matrix = Matrix::from_rows(lhss.iter()
            .flat_map(|(_, lhs)| lhs.clone()).collect());
        
        RawSoc {
            setup,
            soc,
            lhss,
            rounds,
            cohorts,
            bt_handler: bth,
            sb_handler: sbh,
            ll_handler: llh,
        }

    }




    fn reflective_llh(cipher: &dyn Cipher, nr_rounds: usize) -> ReflectiveLlHandler {

        let forward = extract_ll_matrix(|state: u128| -> u128 {
            cipher.linear_layer(state)
        }, cipher.size());

        let inverse = extract_ll_matrix(|state: u128| -> u128 {
            cipher.linear_layer_inv(state)
        }, cipher.size());

        let reflection = extract_ll_matrix(|state: u128| -> u128 {
            cipher.reflection_layer(state)
        }, cipher.size());


        ReflectiveLlHandler {
            block_size: cipher.size(),
            forward,
            inverse,
            reflection,
            half: nr_rounds / 2,
        }
    }

    fn spn_llh(cipher: &dyn Cipher) -> SpnLlHandler {
        // === Build the LL handler ===
        let forward = extract_ll_matrix(|state: u128| -> u128 {
            cipher.linear_layer(state)
        }, cipher.size());

        SpnLlHandler {
            block_size: cipher.size(),
            forward
        }
    }

    fn make_bth_sbh(cipher: &dyn Cipher, nr_rounds: usize, dl_mode: DLmode) -> (BtHandler, SbHandler) {


        // === Build the Sbox Handler AND BTHandler ===

        let mut sbox_size_in: Vec<Round> = vec![Vec::new(); nr_rounds];
        let mut sbox_size_out: Vec<Round> = vec![Vec::new(); nr_rounds];

        // Caches for BaseTables and GenericShards,
        let mut bt_cache: HashMap<RawTable, Arc<BaseTable>> = HashMap::new();
        let mut gs_cache: HashMap<TableHash, Rc<GenericShard>> = HashMap::new();
        // Vecs keeping track of which BT and GS is used when
        let mut bt_placement: Vec<Vec<Arc<BaseTable>>> = vec![Vec::new(); nr_rounds];
        let mut gs_placement: Vec<Vec<Rc<GenericShard>>> = vec![Vec::new(); nr_rounds];

        // Building the generic shards:
        let mut sbox_pos = 0;
        for r in 0..nr_rounds {
            for _ in 0..cipher.num_sboxes() {
                let sbox = cipher.sbox(sbox_pos);
                sbox_pos += 1;
                sbox_size_in[r].push(sbox.size_in());
                sbox_size_out[r].push(sbox.size_out());

                // Update BT cache as needed
                let raw_table = match dl_mode {
                    DLmode::Differential => sbox.ddt().clone(),
                    DLmode::Linear => CgBuilder::adjust_lat(sbox.lat()),
                        // panic!("Upadte needed, see comment in code"); sbox.lat()}, // FIXME for each entry e => | (2*e) - 2^in_size | // TODO verify fix
                    // Their LAT is different than the one we thought they were using
                };


                // Fill BT cache
                let bt = bt_cache.entry(raw_table.clone())
                    .or_insert_with(|| { Arc::new( BaseTable::try_from(raw_table).unwrap() )} );
                bt_placement[r].push(bt.clone());

                // Fill GS cache
                let gs = gs_cache.entry(bt.table_hashed())
                    .or_insert_with(|| { Rc::new(GenericShard::new(&bt,
                                                                   sbox.size_in(),
                                                                   sbox.size_out()))
                    });
                gs_placement[r].push(gs.clone());
            }
        }

        let bth = BtHandler {
            nr_rounds,
            // OBS, this may break on updates to the underlying trait
            sbox_layer_size: cipher.num_sboxes()*cipher.sbox(0).size_out(),
            bt_placement,
        };

        let sbh = SbHandler {
            num_sboxes: cipher.num_sboxes(),
            sbox_size_in,
            sbox_size_out,
            generic_shards: gs_placement,
        };

        (bth, sbh)
    }

    /// The LAT as constructed and used by CryptaGraph is different than what the GenericShard Builder
    /// expects. This fn adjusts the LAT to correspond to the Generic Shards expectations:
    /// > for each entry e => | (2*e) - 2^in_size |
    /// Returns a new LAT
    fn adjust_lat(lat: &Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        // println!("In size = {}", lat.len());
        // println!("Debug: LAT IN:\n{:#?}", lat);

        let in_size = lat.len() as isize;
        let mut res = lat.clone();

        for (i, row) in lat.iter().enumerate() {
            for (j, val) in row.iter().enumerate() {
                res[i][j] = (2*(*val as isize) - in_size).abs() as usize;
            }
        }
        // println!("Debug: LAT OUT:\n{:#?}", &res);
        res
    }
}


// ===================== SBox Handler Build Targets ==================================

pub struct  ReflectiveSbHandler {

}

pub struct SbHandler {
    num_sboxes: usize,
    sbox_size_in: Vec<Vec<usize>>,
    sbox_size_out: Vec<Vec<usize>>,
    // gs_cache: HashMap<TableHash, Rc<GenericShard>>,
    generic_shards: Vec<Vec<Rc<GenericShard>>>,
}

impl SBoxHandler for SbHandler {
    #[inline]
    fn num_sboxes(&self, _round: usize) -> usize {
        self.num_sboxes
    }

    #[inline]
    fn sbox_size_in(&self, round: usize, pos: usize) -> usize {
        self.sbox_size_in[round][pos]
    }

    #[inline]
    fn sbox_size_out(&self, round: usize, pos: usize) -> usize {
        self.sbox_size_out[round][pos]
    }

    #[inline]
    fn bt_generic_shard(&self, round: usize, pos: usize) -> GenericShard {
        (*self.generic_shards[round][pos]).clone()
    }
}




// ================== LL Handler Build Targets ================================

pub struct ReflectiveLlHandler {
    block_size: usize,
    forward: Matrix,
    inverse: Matrix,
    reflection: Matrix,
    half: usize,

}

impl LLHandler for ReflectiveLlHandler {
    fn block_size(&self, _round: usize) -> usize {
        self.block_size
    }

    fn apply_linear_layer(&self, round: usize, state: Vec<Vob<usize>>) -> Vec<Vob<usize>> {
        if round < self.half {
            // println!("Prince A matrix:\n{:?}", self.forward);
            let b = self.forward.left_mul(&Matrix::from_rows(state));
            b.into()
        } else if round == self.half {
            // The 'forward' and 'inverse' are needed to be compatible with CG's implementation
            // of 'reflection'.
            let b = self.forward.left_mul(&Matrix::from_rows(state));
            let b = self.reflection.left_mul(&b);
            let b = self.inverse.left_mul(&b);
            b.into()
        } else {
            let b = self.inverse.left_mul(&Matrix::from_rows(state));
            b.into()
        }
    }
}

// ================================================================================================

pub struct SpnLlHandler {
    block_size: usize,
    forward: Matrix,
}

impl LLHandler for SpnLlHandler {
    fn block_size(&self, _round: usize) -> usize {
        self.block_size
    }

    fn apply_linear_layer(&self, _round: usize, state: Vec<Vob<usize>>) -> Vec<Vob<usize>> {
        let b = self.forward.left_mul(&Matrix::from_rows(state));
        b.into()
    }
}


// ================== BaseTable Handler Build Targets ================================
#[derive(Debug, Clone)]
pub struct BtHandler {
    nr_rounds: usize,
    sbox_layer_size: usize,
    bt_placement:  Vec<Vec<Arc<BaseTable>>>,
}

impl BTHandler for BtHandler {
    fn nr_of_rounds(&self) -> usize {
        self.nr_rounds
    }

    fn bt(&self, round: usize, sbox_pos: usize) -> &BaseTable {
        &(*self.bt_placement[round][sbox_pos])
    }

    fn prob_exponents(&self, round: usize, sbox_pos: usize) -> &BTreeMap<usize, usize> {
        self.bt_placement[round][sbox_pos].prob_exponents()
    }

    fn k(&self, round: usize, sbox_pos: usize) -> f64 {
        self.bt_placement[round][sbox_pos].k()
    }

    fn sbox_layer_size(&self) -> usize {
        self.sbox_layer_size
    }

    fn prob_exponents_for_entry(&self, round: usize, sbox_pos: usize, entry: usize) -> Option<usize> {
        self.bt_placement[round][sbox_pos].prob_exponent_for_entry(entry)
    }
}




// =============================== Linear Layer Matrix Extractor =================================

/// First iteration of the linear layer matrix extractor.
/// In order to correctly build the SoC, we need the linear layer matrix representation.
/// This can be tricky, for various reason. F.ex. it is hidden behind an API, or perhaps it is
/// only indirectly used (meaning that the implementation does not relay on a matrix to do the
/// transformation, but rather on (stepwise) state manipulation through functions or similar).
///
/// This function is designed to extract this linear layer matrix, while not depending on whether
/// the implementation relies on a direct or indirect usage of the ll matrix.
/// The idea is simple. Let A be the ll matrix, which is inaccessible to us. Then we use the facts that:
/// 1) From linear algebra we know that A*Id = A (where Id is the Identity matrix).
/// 2) We also know that any cipher implementing the CG Cipher trait implements a method called
/// linear_layer.
/// If we view this method in abstract terms, we see that it accepts an input state x (as a vector),
/// multiplies x with the ll matrix A, before returning the new post-linear layer state
/// (as a vector) b. I.e. Ax = b.
///
/// We can use these two fact to mine A from the cipher:
/// Break the identity matrix Id into its columns xi. Calling the fn linear_layer on all the xi will
/// result in the output columns bi, which are essentially the columns of A. Reassemble the bi's to
/// recreate A on our side.
///
/// We have now mined A.
///
/// Current limitations:
///     Does not support various linear layer, meaning that we always call linear_function, no-matter
///     which round we're at. (I.e. inverse linear layer of reflective linear layer is never called).
///
fn extract_ll_matrix<F>(lin_fn: F, block_size: usize)  -> Matrix
    where
        F: Fn(u128) -> u128 {


    // Extract columns of A
    let mut bi_s = Vec::new();
    // The x in Ax = b, used to mine A.
    let mut id_elem = 1;

    for _ in 0..block_size {
        let b = lin_fn(id_elem);
        bi_s.push(b);

        id_elem = id_elem << 1;
    }

    let mut vi_s = Vec::with_capacity(bi_s.len());
    // Make into Vob's:
    for bi in bi_s.into_iter() {
        vi_s.push(nn_to_vob(bi));
    }

    // Make into matrix
    // In Crush, a matrix is always stored by rows. This means that we now have the transposed
    // of the matrix we rally want.
    let transposed = algebra::Matrix::from_rows(vi_s);
    // We therefore return the transpose of the transpose
    let res: Vec<Vob> = algebra::transpose(&transposed).into();
    Matrix::from_rows(res.into_iter().take(block_size).collect())
}

/// Turn a u128 into a Vob
fn nn_to_vob(int: u128) -> Vob<usize> {
    // Fixme untested
    let as_bytes = int.to_be_bytes();
    let vob = Vob::from_bytes(&as_bytes);
    let vob: Vob = vob.iter().rev().collect();
    vob
}

#[allow(dead_code)]
/// Turn a Vob of len 128 into a u128
fn vob_to_nn(v: &Vob) -> u128 {
    // FIXME untested
    let bools: Vec<bool> = v.iter().collect();

    let val = bools.iter().enumerate().take(128)
        .fold(0u128, |acc, (idx, x)| { acc | ((*x as u128) << idx)});

    val
}