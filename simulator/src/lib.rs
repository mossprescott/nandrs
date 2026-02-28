/// The primitive gate. All other gates must be built from this.
pub fn nand(a: bool, b: bool) -> bool {
    !(a && b)
}
