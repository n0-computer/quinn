use bencher::{benchmark_group, benchmark_main};
use iroh_quinn_proto::bench_exports::send_buffer::*;

// Since we can't easily access test utilities, this is a minimal benchmark
// that measures the actual problematic operations directly

benchmark_group!(
    benches,
    get_into_many_segments,
    get_loop_many_segments,
);
benchmark_main!(benches);
