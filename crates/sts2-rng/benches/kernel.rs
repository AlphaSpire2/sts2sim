use std::hint::black_box;
use std::time::Instant;

use sts2_rng::Rng;

fn main() {
    const SAMPLES: usize = 5_000_000;
    let mut rng = Rng::new(42);
    let start = Instant::now();
    for _ in 0..SAMPLES {
        black_box(rng.next_int(i32::MAX).unwrap());
    }
    let elapsed = start.elapsed();
    println!("{SAMPLES} counted Xoshiro samples in {elapsed:?}");
}
