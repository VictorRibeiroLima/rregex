use std::fs::File;
use std::path::Path;

use criterion::{Criterion, criterion_group, criterion_main, profiler::Profiler};
use pprof::ProfilerGuard;

fn parse(c: &mut Criterion) {
    let input = "a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z";
    c.bench_function("parse", |b| {
        b.iter(|| rregex::regex::Regex::compile(input).unwrap())
    });
}

fn match_r(c: &mut Criterion) {
    let identifier = "[a-zA-Z_][a-zA-Z0-9_]*";
    let regex = rregex::regex::Regex::compile(identifier).unwrap();
    c.bench_function("match", |b| b.iter(|| regex.find("myVariable").unwrap()));
}

struct FlamegraphProfiler {
    frequency: i32,
    guard: Option<ProfilerGuard<'static>>,
}

impl Profiler for FlamegraphProfiler {
    fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
        self.guard = Some(ProfilerGuard::new(self.frequency).unwrap());
    }

    fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
        std::fs::create_dir_all(benchmark_dir).unwrap();
        let report = self.guard.take().unwrap().report().build().unwrap();
        let file = File::create(benchmark_dir.join("flamegraph.svg")).unwrap();
        report.flamegraph(file).unwrap();
    }
}

fn profiled() -> Criterion {
    Criterion::default().with_profiler(FlamegraphProfiler {
        frequency: 1000,
        guard: None,
    })
}

criterion_group! {
    name = benches;
    config = profiled();
    targets = parse, match_r
}
criterion_main!(benches);
