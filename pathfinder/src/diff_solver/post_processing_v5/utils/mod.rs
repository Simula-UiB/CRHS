pub mod path;

use std::fmt::Error as FmtError;
use std::fmt::Write;

/// bools to less than 8. Takes 'chunk_size' number of bits from 'bits' and turns them into an u8.
    /// Returns a vector with the u8's.
pub fn bools_to_lt8(bits: &[bool], chunk_size: usize) -> Vec<u8> {
    if chunk_size > 8 {
        panic!("Chunk-sizes above 8 won't fit in an u8!");
    }
    let res = bits.chunks(chunk_size)
        .map(|chunk| {
            chunk.iter().enumerate()
                .fold(0u8, |acc, (idx, x)| { acc | ((*x as u8) << idx)} )
        })
        .collect();
    res
}

pub fn bools_to_hex_string(bits: &[bool]) -> Result<String, FmtError> {
    if bits.is_empty() {
        return Ok(String::new());
    }

    let len = bits.len();
    // Chop bits into u64, making it easy to transform them into hex Strings
    let mut u64s = bools_to_u64(bits);

    // We want the LSB to be rightmost and writing to a string starts leftmost (i.e. with
    // MSB). Since MSB is in the last u64 in the Vec, we need to deal with each block of
    // u64 in reverse order.

    let mut buff = String::new();
    // We cannot assume that 64 divides bits.len, meaning that we may not want a width of 16
    // for the u64 block containing the MSB. We need to check and handle accordingly:
    let remainder = len%64;
    if remainder == 0 {
        write!(buff, "{:0>16x}", u64s.pop().expect("Empty inputs should've been returned earlier"))?;
    } else {
        write!(buff, "{:0>w$x}", u64s.pop().expect("Empty inputs should've been returned earlier"),
               w = (len%64)/4)?;
    }
    // Deal with the remaining
    for num in u64s.iter().rev() {
        write!(buff, "{:0>16x}", num)?;
    }
    Ok(buff)
}

// Empty arrays returns an empty string
//
pub fn bools_to_bin_string(bits: &[bool]) -> Result<String, FmtError> {
    if bits.is_empty() {
        return Ok(String::new());
    }

    let len = bits.len();
    // Chop bits into u64, making it easy to transform them into binary Strings
    let mut u64s = bools_to_u64(bits);

    // We want the LSB to be rightmost and writing to a string starts leftmost (i.e. with
    // MSB). Since MSB is in the last u64 in the Vec, we need to deal with each block of
    // u64 in reverse order.

    let mut buff = String::new();
    // We cannot assume that 64 divides bits.len, meaning that we may not want a width of 64
    // for the u64 block containing the MSB. We need to check and handle accordingly:
    let remainder = len%64;
    if remainder == 0 {
        write!(buff, "{:0>64b}", u64s.pop().expect("Empty inputs should've been returned earlier"))?;
    } else {
        write!(buff, "{:0>w$b}", u64s.pop().expect("Empty inputs should've been returned earlier"),
               w = (len%64)/4)?;
    }
    // Deal with the remaining
    for num in u64s.iter().rev() {
        write!(buff, "{:0>64b}", num)?;
    }
    Ok(buff)
}

/// LSB is assumed to be at index 0.
pub fn bools_to_u64(bits: &[bool]) -> Vec<u64> {
    // LSB is assumed to be at index 0.
    bits.chunks(64)
        .map(|chunk| {
            chunk.iter().enumerate()
                .fold(0u64, |acc, (idx, x)| { acc | ((*x as u64) << idx)} )
        })
        .collect()
}




#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use vob::vob;

    use super::*;


    #[test]
    fn test_bools_to_hex_string()  {
        // FIXME insufficient test!
        let mut v = vec![false; 12];
        v[0] = true;
        v[10] = true;
        let actual = bools_to_hex_string(&v).unwrap();
        let expected = "401".to_string();
        assert_eq!(actual, expected);

        let v = vec![];
        let actual = bools_to_hex_string(&v).unwrap();
        assert_eq!(actual, String::new());

        let mut v = vec![false; 65];
        v[1] = true;
        v[64] = true;
        let actual = bools_to_hex_string(&v).unwrap();
        let num = 2_u128.pow(64) + 2;
        let expected = format!("{:0>16x}", num);
        assert_eq!(actual, expected);
    }
}