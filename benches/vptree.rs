use criterion::{black_box, criterion_group, criterion_main, Criterion};
use imanager::VPTree;
use rand::{thread_rng, Rng};

fn float_knn_bench(c: &mut Criterion) {
    let mut rng = thread_rng();
    let mut points = Vec::with_capacity(10000);
    for _ in 0..points.capacity(){
        points.push((rng.gen_range(-100000.0..100000.0), rng.gen_range(-100000.0..100000.0)))
    }
    let points = black_box(points);
    let mut needles = Vec::with_capacity(100);
    for _ in 0..needles.capacity(){
        needles.push(rng.gen_range(0..points.capacity()))
    }
    let needles = black_box(needles);
    c.bench_function("float_knn_benchmark", |b| {
        b.iter(|| {
            let tree = VPTree::new(&points, |a, b| {
                ((a.0 - b.0 as f32).powi(2) + (a.1 - b.1 as f32).powi(2)).sqrt()
            });
            for needle in needles.iter(){
                tree.find_nearest(&points[*needle], 50);
            }
        })
    });
}
criterion_group!(benches, float_knn_bench);
criterion_main!(benches);