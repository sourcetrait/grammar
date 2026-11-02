//! What coding a frame costs: each representative frame encoded and decoded as
//! the codec does it, a fresh coder tree a frame, against coders kept across
//! frames. Every arm's batches interleave in each round, each arm's
//! allocations a frame are counted apart from the timing, and on Linux the
//! host's CPU is read every ten seconds. Run by `cargo bench -p
//! sourcetrait_grammar_protocol --bench frames`; `FRAMES_ROUNDS` sets the
//! rounds, 30 by default, and the report's last line is NUON.
use bitcode::coder::{Decoder, View};
use bitcode::Decode;
use serde::Serialize;
use sourcetrait_common::datum::Nonce;
use sourcetrait_grammar_protocol::{
    DECODE_BUDGET_DEFAULT, DateData, EngineRequest, EngineResponse, ExecuteRequest, FromEngineSys,
    Host, McpFrame, NuVal, ToEngineSys, ValueAnswer, WorkerFrame, decode_payload, payload,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio_util::bytes::Bytes;

/// The time a batch is sized to take.
const BATCH_TARGET: Duration = Duration::from_millis(20);

/// Counts the allocations made while counting is on.
struct Counting;

static COUNTING_ON: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

// SAFETY: each method passes its caller's contract on to `System`'s.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING_ON.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the caller's contract is `System`'s.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is `System`'s.
        unsafe { System.dealloc(p, layout) }
    }

    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING_ON.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the caller's contract is `System`'s.
        unsafe { System.realloc(p, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The allocations, reallocations among them, that `f` makes.
fn allocations(f: impl FnOnce()) -> u64 {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING_ON.store(true, Ordering::Relaxed);
    f();
    COUNTING_ON.store(false, Ordering::Relaxed);
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// A table's rows as an eval returns them: `n` records of six columns.
fn table(n: usize) -> NuVal {
    NuVal::List(
        (0..n)
            .map(|i| {
                NuVal::Record(vec![
                    ("name".into(), NuVal::String(format!("file_{i}.rs"))),
                    (
                        "type".into(),
                        NuVal::String(if i % 7 == 0 { "dir" } else { "file" }.into()),
                    ),
                    ("size".into(), NuVal::Filesize((i as i64 * 1531) % 1_000_000)),
                    (
                        "modified".into(),
                        NuVal::Date(DateData {
                            seconds: 1_790_000_000 + i as i64,
                            seconds_fraction: (i as u32 * 7919) % 1_000_000_000,
                            utc_offset_minus: 0,
                        }),
                    ),
                    ("lines".into(), NuVal::Int((i as i64 * 37) % 5000)),
                    ("tracked".into(), NuVal::Bool(i % 3 != 0)),
                ])
            })
            .collect(),
    )
}

/// A record nested `depth` deep, three fields a level.
fn nested(depth: usize) -> NuVal {
    (0..depth).fold(NuVal::Nothing, |inner, level| {
        NuVal::Record(vec![
            ("level".into(), NuVal::Int(level as i64)),
            ("label".into(), NuVal::String(format!("level {level}"))),
            ("inner".into(), inner),
        ])
    })
}

/// A file's lines as an eval returns them: `n` strings.
fn lines(n: usize) -> NuVal {
    NuVal::List(
        (0..n)
            .map(|i| NuVal::String(format!("    let value_{i} = compute({i}, \"argument\");")))
            .collect(),
    )
}

/// A worker's response frame carrying an eval's `value`.
fn response(value: NuVal) -> WorkerFrame {
    WorkerFrame::Response {
        id: 7,
        response: EngineResponse::Wire(FromEngineSys::Value(ValueAnswer {
            nonce: Nonce::new_unchecked(0x5eed),
            result: Ok(value),
        })),
    }
}

/// The MCP's request frame for a small eval.
fn request() -> McpFrame {
    McpFrame::Request {
        id: 8,
        request: EngineRequest::Wire(ToEngineSys::Execute(ExecuteRequest {
            host: Host::Local,
            nonce: Nonce::new_unchecked(0xfeed),
            definition: "def execute [data: record<n: int>]: nothing -> record<n: int> { $data }"
                .into(),
            data: "{n: 1}".into(),
            timeout: None,
        })),
    }
}

/// A decoder kept across frames, as a buffer keeps one, each frame living as
/// long as the decoder so that no lifetime is erased.
struct Kept<T: for<'a> Decode<'a>> {
    decoder: <T as Decode<'static>>::Decoder,
}

impl<T: for<'a> Decode<'a>> Kept<T> {
    fn new() -> Self {
        Self {
            decoder: Default::default(),
        }
    }

    /// The one frame `bytes` holds, decoded through the kept decoder.
    fn decode(&mut self, mut bytes: &'static [u8]) -> T {
        View::populate(&mut self.decoder, &mut bytes, 1).expect("a frame the bench encoded");
        assert!(bytes.is_empty(), "one frame's bytes");
        // SAFETY: the populate above validated the one value.
        unsafe { Decoder::<'static, T>::decode(&mut self.decoder) }
    }
}

/// One way of coding a frame.
struct Arm {
    name: &'static str,
    run: Box<dyn FnMut()>,
}

/// A representative frame, its encoded size, and its arms.
struct Frame {
    name: &'static str,
    bytes: usize,
    arms: Vec<Arm>,
}

/// `value`'s frame and its arms: encoded fresh, as `payload` does, and through
/// a kept buffer, its bytes copied out as a payload; decoded fresh within the
/// budget, as `decode_payload` does, fresh without it, and through a kept
/// decoder.
fn frame<T>(name: &'static str, value: T) -> Frame
where
    T: bitcode::Encode + for<'a> Decode<'a> + 'static,
{
    // The frame's bytes live as long as the process, as the kept decoder needs.
    let encoded: &'static [u8] = Box::leak(bitcode::encode(&value).into_boxed_slice());
    let value = std::rc::Rc::new(value);
    let encode_fresh = {
        let value = std::rc::Rc::clone(&value);
        move || {
            black_box(payload(&*value));
        }
    };
    let encode_kept = {
        let value = std::rc::Rc::clone(&value);
        let mut buffer = bitcode::Buffer::new();
        move || {
            black_box(Bytes::copy_from_slice(buffer.encode(&*value)));
        }
    };
    let decode_budgeted = move || {
        black_box(decode_payload::<T>(encoded, DECODE_BUDGET_DEFAULT).expect("decodes"));
    };
    let decode_fresh = move || {
        black_box(bitcode::decode::<T>(encoded).expect("decodes"));
    };
    let decode_kept = {
        let mut kept = Kept::<T>::new();
        move || {
            black_box(kept.decode(encoded));
        }
    };
    Frame {
        name,
        bytes: encoded.len(),
        arms: vec![
            Arm { name: "encode_fresh", run: Box::new(encode_fresh) },
            Arm { name: "encode_kept", run: Box::new(encode_kept) },
            Arm { name: "decode_fresh_budgeted", run: Box::new(decode_budgeted) },
            Arm { name: "decode_fresh", run: Box::new(decode_fresh) },
            Arm { name: "decode_kept", run: Box::new(decode_kept) },
        ],
    }
}

/// The host's CPU busy each ten seconds while the rounds run, from
/// `/proc/stat`'s first line, as a percentage of every CPU.
fn sample_cpu(stop: Arc<AtomicBool>, samples: Arc<Mutex<Vec<f64>>>) {
    fn read() -> Option<(u64, u64)> {
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let fields: Vec<u64> = stat
            .lines()
            .next()?
            .split_whitespace()
            .skip(1)
            .filter_map(|f| f.parse().ok())
            .collect();
        let idle = fields.get(3)? + fields.get(4).copied().unwrap_or(0);
        Some((fields.iter().sum(), idle))
    }
    let Some(mut last) = read() else { return };
    while !stop.load(Ordering::Relaxed) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let Some(now) = read() else { return };
        let total = now.0.saturating_sub(last.0);
        let idle = now.1.saturating_sub(last.1);
        if total > 0 {
            let busy = 100.0 * (total - idle.min(total)) as f64 / total as f64;
            samples.lock().expect("the samples' lock").push(busy);
        }
        last = now;
    }
}

#[derive(Serialize)]
struct Report {
    executable: String,
    rounds: usize,
    frames: Vec<FrameReport>,
    cpu_busy_percent: Vec<f64>,
}

#[derive(Serialize)]
struct FrameReport {
    frame: String,
    bytes: u64,
    batch: u64,
    arms: Vec<ArmReport>,
}

#[derive(Serialize)]
struct ArmReport {
    arm: String,
    allocations: u64,
    ns_mean: f64,
    ns_min: f64,
    ns_max: f64,
    ns_range: f64,
    ns_batches: Vec<f64>,
}

fn main() {
    let rounds: usize = std::env::var("FRAMES_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(30);
    let executable = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    println!("executable: {executable}");

    let mut frames = vec![
        frame("table_200x6", response(table(200))),
        frame("nested_record_40", response(nested(40))),
        frame("lines_2000", response(lines(2000))),
        frame("execute_request", request()),
    ];

    // Each arm warmed, then its allocations a frame counted on its own.
    let mut allocs: Vec<Vec<u64>> = Vec::new();
    for frame in &mut frames {
        let mut counts = Vec::new();
        for arm in &mut frame.arms {
            (arm.run)();
            (arm.run)();
            counts.push(allocations(|| (arm.run)()));
        }
        allocs.push(counts);
    }

    // A frame's batch sized so its slowest arm takes about BATCH_TARGET.
    let batches: Vec<u64> = frames
        .iter_mut()
        .map(|frame| {
            let slowest = frame
                .arms
                .iter_mut()
                .map(|arm| {
                    let start = Instant::now();
                    for _ in 0..10 {
                        (arm.run)();
                    }
                    start.elapsed() / 10
                })
                .max()
                .unwrap_or(Duration::from_micros(1));
            (BATCH_TARGET.as_nanos() / slowest.as_nanos().max(1)).max(1) as u64
        })
        .collect();

    let stop = Arc::new(AtomicBool::new(false));
    let samples = Arc::new(Mutex::new(Vec::new()));
    let sampler = {
        let (stop, samples) = (Arc::clone(&stop), Arc::clone(&samples));
        std::thread::spawn(move || sample_cpu(stop, samples))
    };

    // Each round runs every frame's arms, the order rotating a place a round.
    let mut ns: Vec<Vec<Vec<f64>>> = frames
        .iter()
        .map(|f| f.arms.iter().map(|_| Vec::new()).collect())
        .collect();
    for round in 0..rounds {
        for (f, frame) in frames.iter_mut().enumerate() {
            let count = frame.arms.len();
            for k in 0..count {
                let a = (round + k) % count;
                let run = &mut frame.arms[a].run;
                let start = Instant::now();
                for _ in 0..batches[f] {
                    run();
                }
                ns[f][a].push(start.elapsed().as_nanos() as f64 / batches[f] as f64);
            }
        }
    }

    stop.store(true, Ordering::Relaxed);
    sampler.join().expect("the sampler");
    let cpu_busy_percent = samples.lock().expect("the samples' lock").clone();

    let report = Report {
        executable,
        rounds,
        frames: frames
            .iter()
            .enumerate()
            .map(|(f, frame)| FrameReport {
                frame: frame.name.to_owned(),
                bytes: frame.bytes as u64,
                batch: batches[f],
                arms: frame
                    .arms
                    .iter()
                    .enumerate()
                    .map(|(a, arm)| {
                        let batches = &ns[f][a];
                        let min = batches.iter().copied().fold(f64::INFINITY, f64::min);
                        let max = batches.iter().copied().fold(0.0, f64::max);
                        ArmReport {
                            arm: arm.name.to_owned(),
                            allocations: allocs[f][a],
                            ns_mean: batches.iter().sum::<f64>() / batches.len() as f64,
                            ns_min: min,
                            ns_max: max,
                            ns_range: max - min,
                            ns_batches: batches.clone(),
                        }
                    })
                    .collect(),
            })
            .collect(),
        cpu_busy_percent,
    };

    for frame in &report.frames {
        println!("{} ({} bytes, {} a batch)", frame.frame, frame.bytes, frame.batch);
        for arm in &frame.arms {
            println!(
                "  {:<22} {:>6} allocations  mean {:>12.1} ns  min {:>12.1}  max {:>12.1}",
                arm.arm, arm.allocations, arm.ns_mean, arm.ns_min, arm.ns_max
            );
        }
    }
    println!("cpu busy %: {:?}", report.cpu_busy_percent);
    println!(
        "{}",
        nuval::to_nuon(&report, nuval::NuonStyle::Default).expect("the report as NUON")
    );
}
