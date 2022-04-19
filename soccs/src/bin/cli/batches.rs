// ================================================================================================
// ================================== Pre Defined Cipher Setups ===================================
// =================================== As Found in the CG Paper ===================================
// ================================================================================================

// Some ciphers have a lower soft lim the what is given. These are the ones with an 8-bit S-box
// instead of the 4-bit S-boxes the majority of the ciphers use. Lowering the 8-bit ciphers is done
// to stay more in-line with the given soft lim, as we assume that one is given based on the majority
// of S-boxes needs.
//
// These ciphers are:
// AES
// Khazad
// Fly

use soccs::dl::DLmode;

pub struct CipherSetup {
    pub(crate) cipher: String,
    pub(crate) num_rounds: usize,
    pub(crate) soft_lim_e: usize,
    pub(crate) mode: DLmode,
    pub(crate) out_parent_folder: String,
}


pub fn as_from_paper_diff_batch_0(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // EPCBC-48
    settings.push(CipherSetup {
        cipher: "epcbc48".to_string(),
        num_rounds: 13,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "epcbc48".to_string()
    });
    settings.push(CipherSetup {
        cipher: "epcbc48".to_string(),
        num_rounds: 14,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "epcbc48".to_string()
    });

    // EPCBC-96
    settings.push(CipherSetup {
        cipher: "epcbc96".to_string(),
        num_rounds: 20,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "epcbc96".to_string()
    });
    settings.push(CipherSetup {
        cipher: "epcbc96".to_string(),
        num_rounds: 21,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "epcbc96".to_string()
    });

    // AES
    settings.push(CipherSetup {
        cipher: "aes".to_string(),
        num_rounds: 3,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Differential,
        out_parent_folder: "aes".to_string()
    });
    settings.push(CipherSetup {
        cipher: "aes".to_string(),
        num_rounds: 4,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Differential,
        out_parent_folder: "aes".to_string()
    });

    settings
}

pub fn as_from_paper_diff_batch_1(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // GIFT-64
    settings.push(CipherSetup {
        cipher: "gift64".to_string(),
        num_rounds: 12,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "gift64".to_string()
    });
    settings.push(CipherSetup {
        cipher: "gift64".to_string(),
        num_rounds: 13,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "gift64".to_string()
    });


    // Klein
    settings.push(CipherSetup {
        cipher: "klein".to_string(),
        num_rounds: 5,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "klein".to_string()
    });
    settings.push(CipherSetup {
        cipher: "klein".to_string(),
        num_rounds: 6,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "klein".to_string()
    });

    // Led
    settings.push(CipherSetup {
        cipher: "led".to_string(),
        num_rounds: 4,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "led".to_string()
    });

    // Manits7
    settings.push(CipherSetup {
        cipher: "mantis".to_string(),
        num_rounds: 8,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "mantis".to_string()
    });

    // Khazad
    settings.push(CipherSetup {
        cipher: "khazad".to_string(),
        num_rounds: 2,
        soft_lim_e:soft_lim_e.checked_sub(2).unwrap(), // Khazad uses an 8 bit S-box,
        mode: DLmode::Differential,
        out_parent_folder: "khazad".to_string()
    });
    settings.push(CipherSetup {
        cipher: "khazad".to_string(),
        num_rounds: 3,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(), // Khazad uses an 8 bit S-box,
        mode: DLmode::Differential,
        out_parent_folder: "khazad".to_string()
    });

    settings
}

pub fn as_from_paper_diff_batch_2(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // Midori64
    settings.push(CipherSetup {
        cipher: "midori".to_string(),
        num_rounds: 6,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "midori".to_string()
    });
    settings.push(CipherSetup {
        cipher: "midori".to_string(),
        num_rounds: 7,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "midori".to_string()
    });

    // Pride
    settings.push(CipherSetup {
        cipher: "pride".to_string(),
        num_rounds: 15,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "pride".to_string()
    });
    settings.push(CipherSetup {
        cipher: "pride".to_string(),
        num_rounds: 16,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "pride".to_string()
    });

    // Prince, 6 rounds
    settings.push(CipherSetup {
        cipher: "prince".to_string(),
        num_rounds: 2*3,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "prince".to_string()
    });

    // Fly
    settings.push(CipherSetup {
        cipher: "fly".to_string(),
        num_rounds: 8,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Differential,
        out_parent_folder: "fly".to_string()
    });
    settings.push(CipherSetup {
        cipher: "fly".to_string(),
        num_rounds: 9,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Differential,
        out_parent_folder: "fly".to_string()
    });

    settings
}
pub fn as_from_paper_diff_batch_3(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // Prince, 8 rounds
    settings.push(CipherSetup {
        cipher: "prince".to_string(),
        num_rounds: 2*4,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "prince".to_string()
    });

    // Puffin
    settings.push(CipherSetup {
        cipher: "puffin".to_string(),
        num_rounds: 32,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "puffin".to_string()
    });

    // Qarma
    settings.push(CipherSetup {
        cipher: "qarma".to_string(),
        num_rounds: 6,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "qarma".to_string()
    });

    // Rectangle
    settings.push(CipherSetup {
        cipher: "rectangle".to_string(),
        num_rounds: 13,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "rectangle".to_string()
    });
    settings.push(CipherSetup {
        cipher: "rectangle".to_string(),
        num_rounds: 14,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "rectangle".to_string()
    });
    settings.push(CipherSetup {
        cipher: "rectangle".to_string(),
        num_rounds: 15,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "rectangle".to_string()
    });

    // Skinny-64
    settings.push(CipherSetup {
        cipher: "skinny64".to_string(),
        num_rounds: 8,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "skinny64".to_string()
    });
    settings.push(CipherSetup {
        cipher: "skinny64".to_string(),
        num_rounds: 9,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "skinny64".to_string()
    });

    settings
}

/// Present has shown to use many times as much memory as the average of the others, are is therefore
/// sectioned off into its own batch. This gives more control of when to spend much memory.
pub fn as_from_paper_diff_batch_4(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();
    // Present
    settings.push(CipherSetup {
        cipher: "present".to_string(),
        num_rounds: 15,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "present".to_string()
    });
    settings.push(CipherSetup {
        cipher: "present".to_string(),
        num_rounds: 16,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "present".to_string()
    });
    settings.push(CipherSetup {
        cipher: "present".to_string(),
        num_rounds: 17,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "present".to_string()
    });

    settings
}

#[allow(dead_code)]
pub fn additional_128_versions_diff(soft_lim_e: usize) -> Vec<CipherSetup> {
    // Adding GIFT 128 and Skinny 128 last, as they is not in the paper
    let mut settings = vec![];
    // GIFT-128
    settings.push(CipherSetup {
        cipher: "gift128".to_string(),
        num_rounds: 12,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "gift128".to_string()
    });
    settings.push(CipherSetup {
        cipher: "gift128".to_string(),
        num_rounds: 13,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "gift128".to_string()
    });

    // Skinny-128
    settings.push(CipherSetup {
        cipher: "skinny128".to_string(),
        num_rounds: 8,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "skinny128".to_string()
    });
    settings.push(CipherSetup {
        cipher: "skinny128".to_string(),
        num_rounds: 9,
        soft_lim_e,
        mode: DLmode::Differential,
        out_parent_folder: "skinny128".to_string()
    });

    settings
}




// =================================================================================================
// =================================================================================================
// ======================================== Linear =================================================
// =================================================================================================
// =================================================================================================




pub fn as_from_paper_lin_batch_0(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // EPCBC-48
    settings.push(CipherSetup {
        cipher: "epcbc48".to_string(),
        num_rounds: 15,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "epcbc48".to_string()
    });
    settings.push(CipherSetup {
        cipher: "epcbc48".to_string(),
        num_rounds: 16,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "epcbc48".to_string()
    });

    // EPCBC-96
    settings.push(CipherSetup {
        cipher: "epcbc96".to_string(),
        num_rounds: 31,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "epcbc96".to_string()
    });
    settings.push(CipherSetup {
        cipher: "epcbc96".to_string(),
        num_rounds: 32,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "epcbc96".to_string()
    });

    // AES
    settings.push(CipherSetup {
        cipher: "aes".to_string(),
        num_rounds: 3,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Linear,
        out_parent_folder: "aes".to_string()
    });
    settings.push(CipherSetup {
        cipher: "aes".to_string(),
        num_rounds: 4,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Linear,
        out_parent_folder: "aes".to_string()
    });

    settings
}

pub fn as_from_paper_lin_batch_1(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // GIFT-64
    settings.push(CipherSetup {
        cipher: "gift64".to_string(),
        num_rounds: 11,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "gift64".to_string()
    });
    settings.push(CipherSetup {
        cipher: "gift64".to_string(),
        num_rounds: 12,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "gift64".to_string()
    });


    // Klein
    settings.push(CipherSetup {
        cipher: "klein".to_string(),
        num_rounds: 5,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "klein".to_string()
    });
    settings.push(CipherSetup {
        cipher: "klein".to_string(),
        num_rounds: 6,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "klein".to_string()
    });

    // Led
    settings.push(CipherSetup {
        cipher: "led".to_string(),
        num_rounds: 4,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "led".to_string()
    });

    // Manits7
    settings.push(CipherSetup {
        cipher: "mantis".to_string(),
        num_rounds: 8,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "mantis".to_string()
    });

    // Khazad
    settings.push(CipherSetup {
        cipher: "khazad".to_string(),
        num_rounds: 2,
        soft_lim_e:soft_lim_e.checked_sub(2).unwrap(), // Khazad uses an 8 bit S-box,
        mode: DLmode::Linear,
        out_parent_folder: "khazad".to_string()
    });
    settings.push(CipherSetup {
        cipher: "khazad".to_string(),
        num_rounds: 3,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(), // Khazad uses an 8 bit S-box,
        mode: DLmode::Linear,
        out_parent_folder: "khazad".to_string()
    });

    settings
}

pub fn as_from_paper_lin_batch_2(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // Midori64
    settings.push(CipherSetup {
        cipher: "midori".to_string(),
        num_rounds: 5,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "midori".to_string()
    });
    settings.push(CipherSetup {
        cipher: "midori".to_string(),
        num_rounds: 6,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "midori".to_string()
    });

    // Pride
    settings.push(CipherSetup {
        cipher: "pride".to_string(),
        num_rounds: 15,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "pride".to_string()
    });
    settings.push(CipherSetup {
        cipher: "pride".to_string(),
        num_rounds: 16,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "pride".to_string()
    });

    // Prince, 6 rounds
    settings.push(CipherSetup {
        cipher: "prince".to_string(),
        num_rounds: 2*3,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "prince".to_string()
    });

    // Fly
    settings.push(CipherSetup {
        cipher: "fly".to_string(),
        num_rounds: 8,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap(),
        mode: DLmode::Linear,
        out_parent_folder: "fly".to_string()
    });
    settings.push(CipherSetup {
        cipher: "fly".to_string(),
        num_rounds: 9,
        soft_lim_e: soft_lim_e.checked_sub(2).unwrap()  ,
        mode: DLmode::Linear,
        out_parent_folder: "fly".to_string()
    });

    settings
}
pub fn as_from_paper_lin_batch_3(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();

    // Prince, 8 rounds
    settings.push(CipherSetup {
        cipher: "prince".to_string(),
        num_rounds: 2*4,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "prince".to_string()
    });

    // Puffin
    settings.push(CipherSetup {
        cipher: "puffin".to_string(),
        num_rounds: 32,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "puffin".to_string()
    });

    // Qarma
    settings.push(CipherSetup {
        cipher: "qarma".to_string(),
        num_rounds: 6,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "qarma".to_string()
    });

    // Rectangle
    settings.push(CipherSetup {
        cipher: "rectangle".to_string(),
        num_rounds: 12,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "rectangle".to_string()
    });
    settings.push(CipherSetup {
        cipher: "rectangle".to_string(),
        num_rounds: 13,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "rectangle".to_string()
    });
    settings.push(CipherSetup {
        cipher: "rectangle".to_string(),
        num_rounds: 14,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "rectangle".to_string()
    });

    // Skinny-64
    settings.push(CipherSetup {
        cipher: "skinny64".to_string(),
        num_rounds: 8,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "skinny64".to_string()
    });
    settings.push(CipherSetup {
        cipher: "skinny64".to_string(),
        num_rounds: 9,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "skinny64".to_string()
    });

    settings
}

/// Present has shown to use many times as much memory as the average of the others, are is therefore
/// sectioned off into its own batch. This gives more control of when to spend much memory.
pub fn as_from_paper_lin_batch_4(soft_lim_e: usize) -> Vec<CipherSetup> {
    let mut settings = Vec::new();
    // Present
    settings.push(CipherSetup {
        cipher: "present".to_string(),
        num_rounds: 23,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "present".to_string()
    });
    settings.push(CipherSetup {
        cipher: "present".to_string(),
        num_rounds: 24,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "present".to_string()
    });
    settings.push(CipherSetup {
        cipher: "present".to_string(),
        num_rounds: 25,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "present".to_string()
    });

    settings
}

#[allow(dead_code)]
pub fn additional_128_versions_lin(soft_lim_e: usize) -> Vec<CipherSetup> {
    // Adding GIFT 128 and Skinny 128 last, as they is not in the paper
    let mut settings = vec![];
    // GIFT-128
    settings.push(CipherSetup {
        cipher: "gift128".to_string(),
        num_rounds: 12,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "gift128".to_string()
    });
    settings.push(CipherSetup {
        cipher: "gift128".to_string(),
        num_rounds: 13,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "gift128".to_string()
    });

    // Skinny-128
    settings.push(CipherSetup {
        cipher: "skinny128".to_string(),
        num_rounds: 8,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "skinny128".to_string()
    });
    settings.push(CipherSetup {
        cipher: "skinny128".to_string(),
        num_rounds: 9,
        soft_lim_e,
        mode: DLmode::Linear,
        out_parent_folder: "skinny128".to_string()
    });

    settings
}