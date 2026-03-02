use criterion::{criterion_group, criterion_main};
use noq_proto::bench_exports::send_buffer_benches::*;

// Since we can't easily access test utilities, this is a minimal benchmark
// that measures the actual problematic operations directly

criterion_group!(benches, get_into_many_segments, get_loop_many_segments,);
criterion_main!(benches);
