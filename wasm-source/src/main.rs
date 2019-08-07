fn main() {}

// no_mangle is necessary to retain the name and be able to invoke it by that in the WASM runtime
#[no_mangle]
fn add() -> isize {
    5 + 7
}
