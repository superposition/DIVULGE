// The buffer begins at file offset zero. No address-to-file mapping is known.
use divulge::domain::{FileOffset, read_u16_le};

fn main() {
    let bytes = [0x34, 0x12];
    println!("{:?}", read_u16_le(&bytes, FileOffset(0)));
}
