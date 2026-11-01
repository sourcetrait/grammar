//! A session's opening exchange: the MCP's hello, the worker's reply, both checks.
use crate::*;

/// The protocol version a hello names.
#[cereal::derived(Data, Copy, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// The version this build speaks, v1.0.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion { major: 1, minor: 0 };

/// How long a worker waits for the MCP's hello once its start is done.
pub const HELLO_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// How long the MCP waits for a worker's reply, its start among it.
pub const REPLY_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);

/// How long an SSH session may take to connect and authenticate.
pub const AUTH_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// The bounds a hello or its reply decodes within, sized to the hello's frame.
pub const HELLO_BUDGET: DecodeBudget = DecodeBudget {
    elements: 64,
    heap: HELLO_FRAME_MAX as u64,
    depth: 8,
};

/// A worker's namespace: its id and its namespace.
#[cereal::derived(Data, Eq, Hash)]
pub struct Instance {
    pub id: String,
    pub namespace: String,
}

/// Whether `name` is a valid id, namespace, or engine alias.
pub fn valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl Instance {
    /// An instance of `id` and `namespace`, each refused unless valid, never sanitized.
    pub fn new(id: &str, namespace: &str) -> Result<Self, TransportError> {
        let instance = Self {
            id: id.to_string(),
            namespace: namespace.to_string(),
        };
        instance.check()?;
        Ok(instance)
    }

    /// The instance refused unless its id and namespace are both valid.
    pub fn check(&self) -> Result<(), TransportError> {
        for (which, name) in [("id", &self.id), ("namespace", &self.namespace)] {
            if !valid_name(name) {
                return Err(TransportError::InvalidFrame {
                    reason: format!("the {which} `{name}` is not a valid instance name"),
                });
            }
        }
        Ok(())
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.id, self.namespace)
    }
}

/// The bounds a hello negotiates, each the least of both ends'.
#[cereal::derived(Data, Copy, Eq)]
pub struct Limits {
    /// The frame ceiling after the hellos, both ways.
    pub frame: u32,
    /// The heap one frame's payload decodes within.
    pub decode_heap: u64,
    /// The evals, reruns, and calls in flight at once.
    pub calls: u32,
    /// The other requests in flight at once beside them.
    pub control: u32,
}

/// The bounds before negotiation.
pub const LIMITS_DEFAULT: Limits = Limits {
    frame: FRAME_DEFAULT,
    decode_heap: DECODE_BUDGET_DEFAULT.heap,
    calls: 32,
    control: 4,
};

/// The bounds no negotiation passes.
pub const LIMITS_HARD: Limits = Limits {
    frame: FRAME_HARD,
    decode_heap: DECODE_HEAP_HARD,
    calls: 128,
    control: 8,
};

impl Limits {
    /// Each bound by its name, widened.
    fn bounds(&self) -> [(&'static str, u64); 4] {
        [
            ("frame", u64::from(self.frame)),
            ("decode_heap", self.decode_heap),
            ("calls", u64::from(self.calls)),
            ("control", u64::from(self.control)),
        ]
    }

    /// Each bound the least of this one's and `other`'s.
    pub fn least(self, other: Limits) -> Limits {
        Limits {
            frame: self.frame.min(other.frame),
            decode_heap: self.decode_heap.min(other.decode_heap),
            calls: self.calls.min(other.calls),
            control: self.control.min(other.control),
        }
    }

    /// The first bound at zero, by its name.
    pub fn zero(&self) -> Option<&'static str> {
        self.bounds().into_iter().find(|(_, value)| *value == 0).map(|(name, _)| name)
    }

    /// The first bound past `other`'s: its name, this value, and `other`'s.
    pub fn past(&self, other: &Limits) -> Option<(&'static str, u64, u64)> {
        self.bounds()
            .into_iter()
            .zip(other.bounds())
            .find(|((_, mine), (_, theirs))| mine > theirs)
            .map(|((name, mine), (_, theirs))| (name, mine, theirs))
    }

    /// The budget a frame after the hellos decodes within.
    pub fn decode_budget(&self) -> DecodeBudget {
        DecodeBudget {
            heap: self.decode_heap,
            ..DECODE_BUDGET_DEFAULT
        }
    }
}

/// The MCP's first frame: what it speaks, the namespace it asks for, its bounds.
#[cereal::derived(Data, Eq)]
pub struct ClientHello {
    pub version: ProtocolVersion,
    pub schema: [u8; 32],
    pub codec: String,
    /// The namespace the MCP means to reach.
    pub expected: Instance,
    /// The bounds the MCP asks for.
    pub requested: Limits,
}

impl ClientHello {
    /// The hello this build sends for `expected`, asking `requested`.
    pub fn new(expected: Instance, requested: Limits) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            schema: SCHEMA_ID,
            codec: CODEC_ID.to_string(),
            expected,
            requested,
        }
    }
}

/// The worker's acceptance: who it is, and the bounds the session holds to.
#[cereal::derived(Data, Eq)]
pub struct WorkerHello {
    pub version: ProtocolVersion,
    pub schema: [u8; 32],
    pub codec: String,
    pub instance: Instance,
    /// The worker's effective Unix UID.
    pub uid: u32,
    /// The worker's process id.
    pub pid: u32,
    /// The worker's nom, drawn at its start.
    pub nom: String,
    /// The negotiated bounds.
    pub effective: Limits,
}

/// The worker's first frame: the hello accepted, or refused.
#[cereal::derived(Data)]
pub enum HelloReply {
    Accepted(Box<WorkerHello>),
    Refused(TransportError),
}

/// The version, schema, and codec a peer named, held to this build's.
fn agree(version: ProtocolVersion, schema: &[u8; 32], codec: &str) -> Result<(), TransportError> {
    if version.major != PROTOCOL_VERSION.major {
        return Err(TransportError::VersionMismatch {
            expected: PROTOCOL_VERSION.to_string(),
            found: version.to_string(),
        });
    }
    if *schema != SCHEMA_ID {
        return Err(TransportError::SchemaMismatch {
            expected: hex(&SCHEMA_ID),
            found: hex(schema),
        });
    }
    check_codec(codec).map_err(|e| e.transport())
}

/// Bytes in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// What a worker answers a hello from: its identity and its bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerProfile {
    pub instance: Instance,
    pub uid: u32,
    pub pid: u32,
    pub nom: String,
    /// The bounds the worker allows, held to `LIMITS_HARD` as it negotiates.
    pub limits: Limits,
}

impl WorkerProfile {
    /// The worker's check of `hello`: its negotiated limits, or why it is refused.
    pub fn check(&self, hello: &ClientHello) -> Result<Limits, TransportError> {
        agree(hello.version, &hello.schema, &hello.codec)?;
        hello.expected.check()?;
        if hello.expected != self.instance {
            return Err(TransportError::WrongWorker {
                expected: hello.expected.to_string(),
                found: self.instance.to_string(),
            });
        }
        if let Some(name) = hello.requested.zero() {
            return Err(TransportError::InvalidFrame {
                reason: format!("the hello requests a {name} of zero"),
            });
        }
        Ok(hello.requested.least(self.limits).least(LIMITS_HARD))
    }

    /// The worker's reply to `hello`: accepted under the negotiated limits, or refused.
    pub fn reply(&self, hello: &ClientHello) -> HelloReply {
        match self.check(hello) {
            Ok(effective) => HelloReply::Accepted(Box::new(WorkerHello {
                version: PROTOCOL_VERSION,
                schema: SCHEMA_ID,
                codec: CODEC_ID.to_string(),
                instance: self.instance.clone(),
                uid: self.uid,
                pid: self.pid,
                nom: self.nom.clone(),
                effective,
            })),
            Err(refusal) => HelloReply::Refused(refusal),
        }
    }
}

/// The MCP's check of the worker's reply to `hello`: the worker it reached.
pub fn check_reply(reply: HelloReply, hello: &ClientHello) -> Result<WorkerHello, TransportError> {
    let worker = match reply {
        HelloReply::Accepted(worker) => *worker,
        HelloReply::Refused(refusal) => return Err(refusal),
    };
    agree(worker.version, &worker.schema, &worker.codec)?;
    worker.instance.check()?;
    if worker.instance != hello.expected {
        return Err(TransportError::WrongWorker {
            expected: hello.expected.to_string(),
            found: worker.instance.to_string(),
        });
    }
    if !valid_peer_id(&worker.nom) {
        return Err(TransportError::InvalidFrame {
            reason: "the worker's nom isn't base62".to_string(),
        });
    }
    if let Some((name, effective, requested)) = worker.effective.past(&hello.requested) {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker raised {name} to {effective} past the {requested} requested"),
        });
    }
    if let Some((name, effective, hard)) = worker.effective.past(&LIMITS_HARD) {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker granted {name} {effective} past the hard ceiling {hard}"),
        });
    }
    if let Some(name) = worker.effective.zero() {
        return Err(TransportError::InvalidFrame {
            reason: format!("the worker set {name} to zero"),
        });
    }
    Ok(worker)
}
