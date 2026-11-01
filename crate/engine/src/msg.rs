use crate::*;

/// What the engine's controller sends: a wire request, its own, or a host answer.
#[cereal::derived(Data)]
pub enum ToEngine {
    Wire(ToEngineSys),
    Purview(PurviewRequest),
    Host(HostAnswer),
}

/// What the engine sends its controller: the answers, or a host request.
#[cereal::derived(Data)]
pub enum FromEngine {
    Wire(FromEngineSys),
    Purview(PurviewAnswer),
    Host(HostAsk),
}

/// What an eval asks of the process hosting it.
#[cereal::derived(Data)]
pub enum HostAsk {
    /// A body's packet for the Channel.
    ChannelSend(ChannelSendData),
    /// The stop a spam verdict made, for the error's emergency.
    Stopped(ChannelSpamErrorEmergency),
    /// A body's message for a linked peer, its files named.
    RemoteSend(RemoteSendData),
    /// The configuration in force.
    Config,
    /// A setting held for a process's lifetime.
    Pin(PinAsk),
}

/// A pin's dotted key, the process it follows, and the value it holds.
#[cereal::derived(Data)]
pub struct PinAsk {
    pub key: String,
    pub pid: u32,
    pub value: f64,
}

impl subsys::Ask<EngineSystem> for HostAsk {
    type Answer = HostAnswer;
}

impl From<HostAsk> for FromEngine {
    fn from(v: HostAsk) -> Self {
        Self::Host(v)
    }
}

impl TryFrom<ToEngine> for HostAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: ToEngine) -> subsys::SubsysResult<Self> {
        match v {
            ToEngine::Host(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl From<PurviewRequest> for ToEngine {
    fn from(v: PurviewRequest) -> Self {
        Self::Purview(v)
    }
}

impl subsys::Request<EngineSystem> for PurviewRequest {
    type ResponseType = PurviewAnswer;
}

impl TryFrom<FromEngine> for PurviewAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(answer) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// Each purview request's way into `ToEngine`, and the answer it names.
macro_rules! purview_request {
    ($($ty:ty => $variant:ident, $answer:ty),* $(,)?) => {$(
        impl From<$ty> for ToEngine {
            fn from(v: $ty) -> Self { Self::Purview(PurviewRequest::$variant(v)) }
        }

        impl subsys::Request<EngineSystem> for $ty {
            type ResponseType = $answer;
        }
    )*};
}

purview_request! {
    PurviewListRequest => List, Result<Vec<PurviewView>, RuntimeError>,
    PurviewConfigureRequest => Configure, Result<Vec<PurviewRow>, RuntimeError>,
    PurviewResolveRequest => Resolve, Result<PurviewResolution, RuntimeError>,
    PurviewRigRequest => Rig, Result<Vec<PurviewRow>, RuntimeError>,
}

impl TryFrom<FromEngine> for Result<Vec<PurviewView>, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(PurviewAnswer::Listed(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for Result<Vec<PurviewRow>, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(PurviewAnswer::Rows(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for Result<PurviewResolution, RuntimeError> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Purview(PurviewAnswer::Resolved(answer)) => Ok(answer),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

/// Each wire request's way into `ToEngine`.
macro_rules! wire_request {
    ($($ty:ty => $answer:ty),* $(,)?) => {$(
        impl From<$ty> for ToEngine {
            fn from(v: $ty) -> Self { Self::Wire(v.into()) }
        }

        impl subsys::Request<EngineSystem> for $ty {
            type ResponseType = $answer;
        }
    )*};
}

wire_request! {
    ExecuteRequest => ValueAnswer,
    RerunRequest => ValueAnswer,
    CallRequest => ValueAnswer,
    ControlRequest => EngineResult<ControlAnswer>,
}

impl TryFrom<FromEngine> for ValueAnswer {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Wire(answer) => answer.try_into(),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}

impl TryFrom<FromEngine> for EngineResult<ControlAnswer> {
    type Error = subsys::SubsysError;

    fn try_from(v: FromEngine) -> subsys::SubsysResult<Self> {
        match v {
            FromEngine::Wire(answer) => answer.try_into(),
            _ => Err(subsys::SubsysError::ResponseType),
        }
    }
}
