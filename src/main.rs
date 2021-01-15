use imanager::VPTree;
use rand::{thread_rng, Rng};
use std::fs::File;
use std::io::prelude::*;
const VPTREE_DATA_PATH: &'static str = "examples/data/bench/vptree_data.bin";

fn _create_vptree_data() {
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

fn main() {
    let vptree_data = std::fs::read(VPTREE_DATA_PATH).unwrap();
    let (points, needles): (Vec<(f32, f32)>, Vec<usize>) =
        bincode::deserialize(&vptree_data).unwrap();
    let tree = VPTree::new(&points, |a, b| {
        ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
    });
    //println!("Tree root's radius: {}", tree.nodes[0].radius);
    for k in 1..100 {
        let average_distance: f32 = needles
            .iter()
            .map(|needle| {
                tree.find_k_nearest_neighbors(&points[*needle], k)
                    .into_iter()
                    .map(|(distance, _)| distance)
                    .sum::<f32>()
                    / k as f32
            })
            .sum::<f32>()
            / needles.len() as f32;
        println!(
            "Average distance for k={}, as a percentage of root's radius: {}%",
            k,
            average_distance // / tree.nodes[0].radius * 100.0
        );
    }
}
