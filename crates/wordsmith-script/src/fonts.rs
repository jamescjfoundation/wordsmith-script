// SPDX-License-Identifier: MIT OR Apache-2.0
//! The script's typeface, Courier Prime (SIL Open Font License, `assets/fonts/OFL-CourierPrime.txt`),
//! built into the app and added to the core's font set (`jcj_layout::fonts`) the first time a
//! script is made, laid out or printed. Pages need it to keep ten characters to the inch and six
//! lines to the inch (decision 0009).

use std::sync::Once;

/// The family name the screenplay styles use.
pub const FAMILY: &str = "Courier Prime";

const FACES: [&[u8]; 4] = [
    include_bytes!("../../../assets/fonts/CourierPrime-Regular.ttf"),
    include_bytes!("../../../assets/fonts/CourierPrime-Bold.ttf"),
    include_bytes!("../../../assets/fonts/CourierPrime-Italic.ttf"),
    include_bytes!("../../../assets/fonts/CourierPrime-BoldItalic.ttf"),
];

/// Add Courier Prime to the font set (once per process).
pub fn ensure() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for face in FACES {
            jcj_layout::fonts::register_font(face.to_vec());
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn courier_prime_is_in_the_font_set() {
        super::ensure();
        assert!(jcj_layout::fonts::has_family(super::FAMILY));
    }
}
