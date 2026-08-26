// 벤치 — 측정기도 코퍼스도 계약이 소유한다. 이 파일은 이 유닛의 미러를 그 측정기에 세울 뿐이다.
//   SOKSAK_BENCH_OUT=<dir> cargo test --release --test bench -- --ignored --nocapture
//
// ④ 메모리 축이 재는 것은 미러가 붙든 힙이다 — 계약의 계수 할당자를 이 바이너리에 끼운다
// (RSS 가 아니라 순 할당 바이트를 재는 이유는 계약 bench 모듈에 적혀 있다).
#[global_allocator]
static ALLOC: soksak_contract_terminal::bench::CountingAlloc =
    soksak_contract_terminal::bench::CountingAlloc::new();

mod common;

use std::time::{Duration, Instant};

use soksak_contract_terminal::Fixture;
use soksak_sidecar_terminal_vt100::Mirror;

#[test]
#[ignore]
fn bench() {
    let report = soksak_contract_terminal::bench::run::<common::SidecarMirror>(
        "soksak-sidecar-terminal-vt100",
    );
    println!("{}", report.to_json());
    // 기록이 판정보다 먼저다. 떨어진 유닛의 숫자야말로 표에 가장 있어야 할 숫자인데, 판정을
    // 먼저 하면 그 유닛은 아무 기록도 남기지 못하고 사라진다.
    if let Ok(dir) = std::env::var("SOKSAK_BENCH_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("vt100.bench.json"), report.to_json()).expect("write");
    }
    // 예산은 게이트다(SPEC.md §14.2) — 어기면 여기서 떨어진다. 후보끼리 견주지 않는다:
    // 이 유닛이 잰 수요와 이 유닛의 성적만 본다.
    soksak_contract_terminal::bench::assert_within_budget(&report);
}

// frame 예산 — 80×24 의 무거운 화면(트루컬러 wide 행, 1000행 스크롤백)에서 frame_at(0) 의 3회
// 중앙값이 2 ms 이하. release 게이트다: debug 빌드는 측정이 아니라 건너뛴다.
#[test]
fn frame_at_zero_is_within_two_milliseconds() {
    if cfg!(debug_assertions) {
        eprintln!("frame budget is a release-build gate; skipped in debug");
        return;
    }
    let mut mirror = Mirror::new(80, 24);
    mirror.feed(&Fixture::PrivateModes.stream());
    let mut samples: Vec<Duration> = (0..3)
        .map(|_| {
            let started = Instant::now();
            let frame = mirror.frame_at(0);
            std::hint::black_box(frame);
            started.elapsed()
        })
        .collect();
    samples.sort();
    assert!(
        samples[1] <= Duration::from_millis(2),
        "frame_at(0) median {:?} exceeds the 2 ms budget (samples {samples:?})",
        samples[1]
    );
}
