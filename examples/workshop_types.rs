//! Run with: cargo run --example workshop_types
use divulge::domain::{FileOffset, MappedAddress, Observation, read_u16_le};

fn main() {
    let bytes: [u8; 2] = [0x34, 0x12];
    let observation = Observation {
        offset: FileOffset(0),
        bytes: bytes.to_vec(),
        description: String::from("Synthetic workshop fixture, not firmware evidence"),
    };

    println!("Observed: {:02x?}", observation.bytes);
    match read_u16_le(&observation.bytes, observation.offset) {
        Ok(value) => println!("As a little-endian u16: {value}"),
        Err(reason) => println!("Could not interpret: {reason}"),
    }

    let destination: Option<MappedAddress> = None;
    println!("Known branch destination: {destination:?}");
    println!("What this value represents is still unknown.");

    // Try this: the compiler keeps a file coordinate separate from a memory coordinate.
    // let wrong: FileOffset = MappedAddress(0x0200_0120);
    // Try truncating the fixture to one byte: an error is different from an unknown destination.
}
