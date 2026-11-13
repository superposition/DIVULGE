//! Checkpoint 1: the bytes stay the same when their interpretation changes.
fn main() {
    let bytes: [u8; 2] = [0x34, 0x12];
    let view: &[u8] = &bytes;
    let little: u16 = u16::from_le_bytes(bytes);
    let big: u16 = u16::from_be_bytes(bytes);
    println!("Synthetic bytes: {view:02x?}");
    println!("little-endian u16: {little}");
    println!("big-endian u16: {big}");
    println!("Intended meaning: unknown");
}
