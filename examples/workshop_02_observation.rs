//! Checkpoint 2: use the same Observation struct as Harness Insight.
use divulge::domain::{FileOffset, Observation};

fn main() {
    let observation = Observation {
        offset: FileOffset(0),
        bytes: vec![0x34, 0x12],
        description: String::from("Synthetic teaching bytes"),
    };
    println!(
        "file+0x{:x}: {:02x?}",
        observation.offset.0, observation.bytes
    );
    println!(
        "{} / {} bytes",
        observation.description,
        observation.bytes.len()
    );
    println!("Vec<u8> owns the bytes; String owns the description.");
}
