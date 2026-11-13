//! Checkpoint 4: a missing destination is different from a failed read.
use divulge::domain::{FileOffset, MappedAddress, read_u16_le};

fn main() {
    let bytes = [0x34, 0x12];
    let complete: Result<u16, &str> = read_u16_le(&bytes, FileOffset(0));
    let short: Result<u16, &str> = read_u16_le(&bytes[..1], FileOffset(0));
    let destination: Option<MappedAddress> = None;
    println!("Two bytes: {complete:?}");
    println!("One byte: {short:?}");
    match destination {
        Some(address) => println!("Destination: 0x{:x}", address.0),
        None => println!("Destination: unknown; no address was invented"),
    }
}
