use alloc::{vec, vec::Vec};

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
    result
}

pub(crate) fn remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
    // This shift-register form avoids allocating one polynomial for every input codeword.
    let mut result = vec![0; divisor.len()];

    for &byte in data {
        let factor = byte ^ result[0];

        result.copy_within(1.., 0);

        *result.last_mut().expect("the divisor is not empty") = 0;

        for (value, &coefficient) in result.iter_mut().zip(divisor) {
            *value ^= multiply(coefficient, factor);
        }
    }

    result
}

#[inline]
fn multiply(x: u8, y: u8) -> u8 {
    // The reduced polynomial uses the low byte of the QR Code primitive polynomial 0x11D.
    let mut product = 0;

    for shift in (0..8).rev() {
        product = (product << 1) ^ ((product >> 7) * 0x1D);

        product ^= ((y >> shift) & 1) * x;
    }

    product
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_i_reed_solomon_codewords() {
        let data = [
            0x10, 0x20, 0x0C, 0x56, 0x61, 0x80, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11, 0xEC, 0x11,
            0xEC, 0x11,
        ];
        assert_eq!(remainder(&data, &divisor(10)), [
            0xA5, 0x24, 0xD4, 0xC1, 0xED, 0x36, 0xC7, 0x87, 0x2C, 0x55
        ]);
    }
}
