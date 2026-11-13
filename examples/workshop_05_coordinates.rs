//! Checkpoint 5: borrowing preserves ownership; newtypes preserve meaning.
use divulge::domain::{FileOffset, MappedAddress, read_u16_le};

fn count(bytes: &[u8]) -> usize {
    bytes.len()
}

fn main() {
    let bytes = vec![0x34, 0x12];
    println!("Borrowed {} bytes", count(&bytes));
    println!("Owner still has: {bytes:02x?}");
    let file_offset = FileOffset(0);
    let declared_address = MappedAddress(0x0200_0120);
    println!("File read: {:?}", read_u16_le(&bytes, file_offset));
    println!("Declared address: 0x{:08x}", declared_address.0);
    println!("These coordinates need a checked map, not a cast.");
}
