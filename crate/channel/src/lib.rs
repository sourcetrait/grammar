pub(crate) mod msg;
pub(crate) mod render;
pub(crate) mod spam;
pub(crate) mod state;
pub(crate) mod system;
#[cfg(test)]
mod tests {
    mod render;
    mod spam;
    mod state;
    mod system;
}

pub(crate) use crate::{
    render::nu_value,
    spam::{Spam, SpamVerdict},
    state::{Channel, UpKind, write_files},
};

pub(crate) use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) use sourcetrait_common::{agnostic, cereal, datum, subsys};

pub(crate) use sourcetrait_grammar_protocol::{
    ChannelEpoch, ChannelMode, ChannelPhase, ChannelSendData, ChannelSpamErrorEmergency,
    ChannelSpamWarningEmergency, ChannelState, EVENT_BYTES_KEY, EVENT_SPILL_KEY, Emergency,
    NOTIFICATION_CAP, NuVal, SYNC_QUEUE_CAP, SpamThresholdsData, reserved_prefix,
};

pub(crate) use nuval::TryIntoNu;

pub(crate) mod nu {
    pub(crate) use nu_protocol::{Record, Span, Value, engine::EngineState};
    pub(crate) use nuon::{ToNuonConfig, to_nuon};
}

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub(crate) mod tk {
    pub(crate) use tokio::{runtime::Handle, task::spawn_blocking};
}

pub use crate::{
    msg::{
        ChannelSendError, ChannelStatus, ChannelSyncError, ChannelVerifyError, CloseHub,
        CloseRequest, Closed, EmitError, EmitRequest, ExpireRequest, FromChannelSys,
        HostRequest, HubNotice, Line, OpenRequest, Opened, ReadyRequest, SendRequest,
        SendVerdict, SpamStop, StatusRequest, StoppedNotice, SyncRequest, Synced,
        ThresholdsRequest, ToChannelSys,
    },
    render::{
        MsgId, escape_line, event_overflows, event_spill_pointer, mint_msg_id, render_nuon,
        render_packet,
    },
    spam::SpamThresholds,
    system::{ChannelHandle, ChannelSysConfig, ChannelSysParams, ChannelSysPaths, ChannelSystem},
};
