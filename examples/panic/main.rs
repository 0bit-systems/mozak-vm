#![cfg_attr(target_os = "zkvm", no_main)]
#![cfg_attr(not(feature = "std"), no_std)]

pub fn main() {
    panic!("Mozak VM panics 😱");
}

guest::entry!(main);
