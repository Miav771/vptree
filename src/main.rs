use rand::{thread_rng, Rng};
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let mut rng = thread_rng();
    let mut points = Vec::with_capacity(100000);
    for _ in 0..points.capacity() {
        points.push((
            rng.gen_range((-1000000.0 as f32)..(1000000.0 as f32)),
            rng.gen_range((-1000000.0 as f32)..(1000000.0 as f32)),
        ))
    }
    let mut needles = Vec::with_capacity(1000);
    for _ in 0..needles.capacity() {
        needles.push(rng.gen_range(0..points.capacity()))
    }
    let data = bincode::serialize(&(points, needles)).unwrap();
    let mut file = File::create("vptree_data.bin").unwrap();
    file.write_all(&data).unwrap();
}
