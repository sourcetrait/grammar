//! A frame's payload decoded within its budget: the retained budget vectors met
//! at their bounds on a worker's stack, an amplified payload refused before its
//! value is allocated, each bound's edge on a frame, each charge held to what
//! the decoded frame keeps, and decodes on threads.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::Path;

use sha2::{Digest, Sha256};
use sourcetrait_common::datum::Nonce;
use sourcetrait_grammar_protocol::{
    ControlAnswer, DECODE_BUDGET_DEFAULT, DecodeBudget, EngineResponse, FrameError, FromEngineSys, NuVal,
    ProcessRow, ValueAnswer, ValueLimitError, ValueLimits, WorkerFrame, check_value, decode_payload,
};

/// A worker's response carrying `result` as a call's value.
fn response(result: NuVal) -> WorkerFrame {
    WorkerFrame::Response {
        id: 1,
        response: EngineResponse::Wire(FromEngineSys::Value(ValueAnswer {
            nonce: Nonce::new_unchecked(7),
            result: Ok(result),
        })),
    }
}

/// A worker's response carrying a control command's answer.
fn control(answer: ControlAnswer) -> WorkerFrame {
    WorkerFrame::Response {
        id: 1,
        response: EngineResponse::Wire(FromEngineSys::Control(Ok(answer))),
    }
}

/// Counts the heap this thread holds and its peak, so a decode's allocation is
/// read apart from other tests' threads.
struct Counting;

std::thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn held(delta: isize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get() + delta;
        live.set(now);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            held(layout.size() as isize);
        }
        p
    }

    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        held(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, layout, size) };
        if !q.is_null() {
            held(size as isize - layout.size() as isize);
        }
        q
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// `f`'s result and the most heap this thread held past its start while it ran.
fn peak<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let start = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(start));
    let result = f();
    let most = PEAK.with(Cell::get) - start;
    (result, most.max(0) as usize)
}

/// A tokio worker thread's stack, which a frame's decode runs on.
const WORKER_STACK: usize = 2 << 20;

/// `f` run on a thread with `stack` bytes of stack.
fn on_stack<R: Send + 'static>(stack: usize, f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::Builder::new().stack_size(stack).spawn(f).expect("a thread").join().expect("its result")
}

/// A value of `depth` lists, one inside the next, around nothing.
fn nested(depth: usize) -> NuVal {
    let mut value = NuVal::Nothing;
    for _ in 0..depth {
        value = NuVal::List(vec![value]);
    }
    value
}

/// The budget vectors `tests/vectors/budget.nuon` retains, encoded from the
/// values the framing test builds them from.
fn budget_vectors() -> Vec<(String, Vec<u8>)> {
    let vectors = [
        ("list_past_elements", NuVal::List(vec![NuVal::Nothing; 100_001])),
        ("record_past_elements", NuVal::Record(vec![(String::new(), NuVal::Nothing); 100_001])),
        ("list_past_depth", nested(65)),
        ("list_deep", nested(10_000)),
    ];
    vectors.into_iter().map(|(name, value)| (name.to_string(), bitcode::encode(&value))).collect()
}

/// The retained vectors' rows, by name: each payload's length and SHA-256.
fn retained() -> Vec<(String, i64, String)> {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/budget.nuon"))
        .expect("the budget vectors");
    let NuVal::List(rows) = NuVal::from_nuon(&text).expect("their NUON") else { panic!("a table") };
    rows.into_iter()
        .map(|row| {
            let NuVal::Record(fields) = row else { panic!("a row") };
            let field = |name: &str| fields.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()).expect(name);
            let (NuVal::String(name), NuVal::Int(bytes), NuVal::String(sha256)) = (field("name"), field("bytes"), field("sha256")) else {
                panic!("a row's columns")
            };
            (name, bytes, sha256)
        })
        .collect()
}

/// A budget's bound past which `payload` was refused, panicking on anything else.
fn refused(payload: &[u8], budget: DecodeBudget) -> (bitcode::Exceeded, u64) {
    match decode_payload::<WorkerFrame>(payload, budget) {
        Err(FrameError::Budget { bound, limit }) => (bound, limit),
        other => panic!("not refused by a bound: {other:?}"),
    }
}

#[test]
fn each_budget_vector_meets_its_bound_on_a_worker_s_stack() {
    // A deep value's encoding and drop recurse, so the vectors build on a large stack.
    let vectors = on_stack(256 << 20, budget_vectors);
    let retained = retained();
    assert_eq!(retained.len(), vectors.len());
    for (name, bytes) in vectors {
        let sha256: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        assert!(
            retained.iter().any(|(n, b, s)| *n == name && *b == bytes.len() as i64 && *s == sha256),
            "{name} is the vector budget.nuon retains",
        );
        let outcome = on_stack(WORKER_STACK, move || {
            decode_payload::<NuVal>(&bytes, DECODE_BUDGET_DEFAULT).map(|value| check_value(&value, ValueLimits::V1))
        });
        match name.as_str() {
            "list_past_elements" | "record_past_elements" => assert!(
                matches!(outcome, Err(FrameError::Budget { bound: bitcode::Exceeded::Elements, limit: 100_000 })),
                "{name}: {outcome:?}",
            ),
            // The decode holds the value's levels with room for the frame's own;
            // the value's check holds its 64.
            "list_past_depth" => assert!(
                matches!(outcome, Ok(Err(ValueLimitError::Depth { limit: 64 }))),
                "{name}: {outcome:?}",
            ),
            // Ten thousand levels: refused at the decode's bound, before the
            // stack runs out.
            "list_deep" => assert!(
                matches!(outcome, Err(FrameError::Budget { bound: bitcode::Exceeded::Depth, limit: 128 })),
                "{name}: {outcome:?}",
            ),
            other => panic!("an unexpected vector {other}"),
        }
    }
    // The deepest value the budget admits decodes, and drops, on a worker's stack.
    let deepest = on_stack(256 << 20, || bitcode::encode(&nested(128)));
    on_stack(WORKER_STACK, move || {
        let value = decode_payload::<NuVal>(&deepest, DECODE_BUDGET_DEFAULT).expect("128 levels");
        drop(value);
    });
}

#[test]
fn an_amplified_payload_is_refused_before_its_value_is_allocated() {
    // A million nothings: half a megabyte on the wire, some 32 MB decoded.
    let bytes = bitcode::encode(&NuVal::List(vec![NuVal::Nothing; 1_000_000]));
    let (decoded, unbudgeted) = peak(|| bitcode::decode::<NuVal>(&bytes).map(drop));
    decoded.expect("bitcode's own decode takes it");
    let (refusal, budgeted) = peak(|| decode_payload::<NuVal>(&bytes, DECODE_BUDGET_DEFAULT).map(drop));
    println!("payload {} bytes; peak heap unbudgeted {unbudgeted}, budgeted {budgeted}", bytes.len());
    assert!(matches!(refusal, Err(FrameError::Budget { bound: bitcode::Exceeded::Elements, .. })), "{refusal:?}");
    assert!(unbudgeted > 32 * bytes.len(), "the payload amplifies {unbudgeted} / {}", bytes.len());
    assert!(budgeted < bytes.len(), "the refusal held {budgeted} bytes of heap");
}

#[test]
fn each_bound_holds_at_its_edge_on_a_frame() {
    let reply = |result| bitcode::encode(&response(result));
    let items = reply(NuVal::List((0..1000).map(NuVal::Int).collect()));
    let elements = DecodeBudget { elements: 1000, ..DECODE_BUDGET_DEFAULT };
    decode_payload::<WorkerFrame>(&items, elements).expect("its 1000 items");
    assert_eq!(refused(&items, DecodeBudget { elements: 999, ..elements }), (bitcode::Exceeded::Elements, 999));

    let deep = reply(nested(10));
    let depth = DecodeBudget { depth: 10, ..DECODE_BUDGET_DEFAULT };
    decode_payload::<WorkerFrame>(&deep, depth).expect("its 10 levels");
    assert_eq!(refused(&deep, DecodeBudget { depth: 9, ..depth }), (bitcode::Exceeded::Depth, 9));

    let text = bitcode::encode(&control(ControlAnswer::Signatures("s".repeat(4096))));
    let heap = DecodeBudget { heap: 4096, ..DECODE_BUDGET_DEFAULT };
    decode_payload::<WorkerFrame>(&text, heap).expect("its 4096 bytes");
    assert_eq!(refused(&text, DecodeBudget { heap: 4095, ..heap }), (bitcode::Exceeded::Heap, 4095));
}

/// The least heap a decode of `payload` passes with: what it is charged.
fn charged(payload: &[u8]) -> u64 {
    let fits = |heap| decode_payload::<WorkerFrame>(payload, DecodeBudget { heap, ..DECODE_BUDGET_DEFAULT }).is_ok();
    let (mut low, mut high) = (0u64, DECODE_BUDGET_DEFAULT.heap);
    assert!(fits(high), "decodes within the default budget");
    while low < high {
        let mid = low + (high - low) / 2;
        if fits(mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low
}

/// The heap the decoded frame keeps, as the allocator counted it.
fn kept(payload: &[u8]) -> u64 {
    let before = LIVE.with(Cell::get);
    let frame = decode_payload::<WorkerFrame>(payload, DECODE_BUDGET_DEFAULT).expect("decoded");
    let after = LIVE.with(Cell::get);
    drop(frame);
    (after - before) as u64
}

#[test]
fn each_frame_s_charge_covers_what_it_keeps() {
    let row = |i: usize| ProcessRow {
        nonce: format!("n{i}"),
        tool: "execute".to_string(),
        started_at: i as u64,
        args: NuVal::Record(vec![("data".to_string(), NuVal::List((0..i as i64).map(NuVal::Int).collect()))]),
        source_nonce: i.is_multiple_of(2).then(|| "s".to_string()),
        path: None,
    };
    let frames = [
        (
            "a table",
            response(NuVal::List(
                (0..50)
                    .map(|i| NuVal::Record(vec![
                        ("name".to_string(), NuVal::String("x".repeat(i % 9))),
                        ("size".to_string(), NuVal::Int(i as i64)),
                        ("raw".to_string(), NuVal::Binary(vec![1; i])),
                    ]))
                    .collect(),
            )),
        ),
        ("a deep value", response(nested(40))),
        ("processes", control(ControlAnswer::Processes((0..20).map(row).collect()))),
    ];
    for (name, frame) in frames {
        let payload = bitcode::encode(&frame);
        let (kept, charged) = (kept(&payload), charged(&payload));
        println!("{name}: kept {kept}, charged {charged}");
        assert!(kept <= charged, "{name}: kept {kept} past its charge {charged}");
    }
}

#[test]
fn decodes_on_threads_keep_their_own_budgets() {
    let threads: Vec<_> = (0..8usize)
        .map(|t| {
            std::thread::spawn(move || {
                let n = 100 + t;
                let payload = bitcode::encode(&response(NuVal::List(vec![NuVal::Bool(true); n])));
                let fits = DecodeBudget { elements: n as u64, ..DECODE_BUDGET_DEFAULT };
                for round in 0..100 {
                    if (round + t).is_multiple_of(2) {
                        decode_payload::<WorkerFrame>(&payload, fits).expect("within its budget");
                    } else {
                        let short = DecodeBudget { elements: n as u64 - 1, ..fits };
                        assert_eq!(refused(&payload, short).0, bitcode::Exceeded::Elements);
                    }
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("a thread's decodes");
    }
}

#[test]
fn a_refused_frame_leaves_the_next_its_whole_budget() {
    let small = bitcode::encode(&response(NuVal::List(vec![NuVal::Nothing; 10])));
    let large = bitcode::encode(&response(NuVal::List(vec![NuVal::Nothing; 11])));
    let budget = DecodeBudget { elements: 10, ..DECODE_BUDGET_DEFAULT };
    for _ in 0..3 {
        assert_eq!(refused(&large, budget).0, bitcode::Exceeded::Elements);
        decode_payload::<WorkerFrame>(&small, budget).expect("its ten");
    }
}
