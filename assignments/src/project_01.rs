#![allow(unused_variables, dead_code, unused_imports)]

use simulator::nand;

pub fn not(a: bool) -> bool {
    todo!()
}

pub fn and(a: bool, b: bool) -> bool {
    todo!()
}

pub fn or(a: bool, b: bool) -> bool {
    todo!()
}

pub fn xor(a: bool, b: bool) -> bool {
    todo!()
}

/// If sel is false, output a. If sel is true, output b.
pub fn mux(a: bool, b: bool, sel: bool) -> bool {
    todo!()
}

/// If sel is false, output (a, false). If sel is true, output (false, b).
pub fn dmux(input: bool, sel: bool) -> (bool, bool) {
    todo!()
}

pub fn not16(a: [bool; 16]) -> [bool; 16] {
    todo!()
}

pub fn and16(a: [bool; 16], b: [bool; 16]) -> [bool; 16] {
    todo!()
}

pub fn or16(a: [bool; 16], b: [bool; 16]) -> [bool; 16] {
    todo!()
}

pub fn mux16(a: [bool; 16], b: [bool; 16], sel: bool) -> [bool; 16] {
    todo!()
}

/// True if any bit in the input is true.
pub fn or8way(input: [bool; 8]) -> bool {
    todo!()
}

pub fn mux4way16(a: [bool; 16], b: [bool; 16], c: [bool; 16], d: [bool; 16], sel: [bool; 2]) -> [bool; 16] {
    todo!()
}

pub fn mux8way16(
    a: [bool; 16], b: [bool; 16], c: [bool; 16], d: [bool; 16],
    e: [bool; 16], f: [bool; 16], g: [bool; 16], h: [bool; 16],
    sel: [bool; 3],
) -> [bool; 16] {
    todo!()
}

pub fn dmux4way(input: bool, sel: [bool; 2]) -> (bool, bool, bool, bool) {
    todo!()
}

pub fn dmux8way(input: bool, sel: [bool; 3]) -> (bool, bool, bool, bool, bool, bool, bool, bool) {
    todo!()
}
