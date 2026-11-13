// Intentional compiler error. Run scripts/check_workshop.py to verify it.
use divulge::domain::{MappedAddress, read_u16_le};

fn main() {
    let bytes = [0x34, 0x12];
    let address = MappedAddress(0x0200_0120);
    println!("{:?}", read_u16_le(&bytes, address));
}
