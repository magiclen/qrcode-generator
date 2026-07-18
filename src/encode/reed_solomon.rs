use alloc::{vec, vec::Vec};

// Powers of the generator element 2 in GF(256) reduced by the QR Code polynomial 0x11D.
// The table is doubled so a sum of two logarithms can index it without a modulo reduction.
static EXP: [u8; 510] = build_exp();
// Logarithms base 2 for every non-zero field element; index 0 is unused and stays 0.
static LOG: [u8; 256] = build_log();

const fn build_exp() -> [u8; 510] {
    let mut table = [0; 510];
    let mut value: u16 = 1;
    let mut index = 0;

    while index < 255 {
        table[index] = value as u8;
        table[index + 255] = value as u8;
        value <<= 1;

        if value & 0x100 != 0 {
            value ^= 0x11D;
        }

        index += 1;
    }

    table
}

const fn build_log() -> [u8; 256] {
    let mut table = [0; 256];
    let mut value: u16 = 1;
    let mut index = 0;

    while index < 255 {
        table[value as usize] = index as u8;
        value <<= 1;

        if value & 0x100 != 0 {
            value ^= 0x11D;
        }

        index += 1;
    }

    table
}

pub(crate) fn divisor(degree: usize) -> Vec<u8> {
    debug_assert!((1..=255).contains(&degree));

    let mut result = vec![0; degree];

    result[degree - 1] = 1;

    // Multiplying by each consecutive field root builds the generator polynomial in place.
    let mut root = 1;

    for _ in 0..degree {
        for index in 0..degree {
            result[index] = multiply(result[index], root);
            if index + 1 < degree {
                result[index] ^= result[index + 1];
            }
        }
        root = multiply(root, 2);
    }

    // The logarithm shortcut in `remainder` needs every generator coefficient to be non-zero.
    debug_assert!(result.iter().all(|&coefficient| coefficient != 0));

    result
}

pub(crate) fn remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
    // This shift-register form avoids allocating one polynomial for every input codeword.
    let mut result = vec![0; divisor.len()];

    for &byte in data {
        let factor = byte ^ result[0];

        result.copy_within(1.., 0);

        *result.last_mut().expect("the divisor is not empty") = 0;

        if factor != 0 {
            let factor_log = usize::from(LOG[usize::from(factor)]);

            // Generator polynomial coefficients are never zero, so their logarithms are all valid.
            for (value, &coefficient) in result.iter_mut().zip(divisor) {
                *value ^= EXP[usize::from(LOG[usize::from(coefficient)]) + factor_log];
            }
        }
    }

    result
}

#[inline]
fn multiply(x: u8, y: u8) -> u8 {
    if x == 0 || y == 0 {
        0
    } else {
        EXP[usize::from(LOG[usize::from(x)]) + usize::from(LOG[usize::from(y)])]
    }
}
