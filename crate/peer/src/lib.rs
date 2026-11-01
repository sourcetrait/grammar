pub(crate) mod codec;
pub(crate) mod link;
pub(crate) mod msg;
pub(crate) mod remotes;
pub(crate) mod system;
pub(crate) mod verify;
#[cfg(test)]
mod tests {
    mod link;
    mod remotes;
    mod system;
}

pub(crate) use crate::{
    codec::{
        AcceptorToInitiator, BitcodeCodec, FileFrame, InitiatorToAcceptor, MsgFrame, RemoteStream,
        check_outbound,
    },
    link::{
        BoundListener, Dial, EndedLink, LinkContext, LinkEvent, LinkKeys, Listen,
        RemoteLinkEntry, Up, bind_listener, ended_link, watch_link,
    },
    remotes::{
        RemoteChange, RemoteEntry, bind_address, dial_address, read_remotes, set_entry,
        unset_entry,
    },
    verify::PublicKeyVerifier,
};

pub(crate) use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs,
    io::{self, Write},
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

pub(crate) use sourcetrait_common::{agnostic, cereal, datum, subsys};

pub(crate) use sourcetrait_grammar_protocol::{
    DECODE_BUDGET_DEFAULT, EVENT_SPILL_DEST, EVENT_SPILL_KEY, NuVal, RemoteChannelEvent,
    RemoteChannelRow, RemoteFailure, RemoteLinkState, RemoteListenResult, RemotePacket,
    RemoteResult, RuntimeError, decode_payload, reserved_prefix, safe_dest, valid_peer_id,
};

pub(crate) use sourcetrait_grammar_channel::{
    event_overflows, event_spill_pointer, mint_msg_id, render_nuon,
};

pub(crate) use nuval::TryFromNu;

pub(crate) use futures_util::{SinkExt, StreamExt};
pub(crate) use tokio_rustls::rustls::pki_types::pem::PemObject;

pub(crate) mod nu {
    pub(crate) use nu_protocol::{Span, Value};
    pub(crate) use nuon::from_nuon;
}

pub(crate) mod ser {
    pub(crate) use ::serde::{Deserialize, Serialize};
}

pub(crate) mod tk {
    pub(crate) use tokio::{
        io::{ReadHalf, WriteHalf, split},
        net::{TcpListener, TcpStream},
        runtime::Handle,
        select, spawn,
        sync::{
            mpsc::{UnboundedReceiver, UnboundedSender, error::SendError, unbounded_channel},
            oneshot,
        },
        task::JoinHandle,
        time::{Duration as TkDuration, timeout},
    };
}

pub(crate) mod tku {
    pub(crate) use tokio_util::bytes::{Bytes, BytesMut};
    pub(crate) use tokio_util::codec::{
        Decoder, Encoder, FramedRead, FramedWrite, LengthDelimitedCodec,
    };
    pub(crate) use tokio_util::sync::CancellationToken;
}

pub(crate) mod tls {
    pub(crate) use tokio_rustls::rustls::pki_types::PrivateKeyDer;
    pub(crate) use tokio_rustls::rustls::{ClientConfig, ServerConfig};
    pub(crate) use tokio_rustls::{TlsAcceptor, TlsConnector, client, server};
}

pub(crate) mod rv {
    pub(crate) use rustls::client::danger::{
        HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
    };
    pub(crate) use rustls::crypto::ring::default_provider;
    pub(crate) use rustls::crypto::{
        CryptoProvider, verify_tls12_signature, verify_tls13_signature,
    };
    pub(crate) use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    pub(crate) use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
    pub(crate) use rustls::{CertificateError, DigitallySignedStruct, DistinguishedName, SignatureScheme};
}

pub use crate::{
    msg::{
        ExpandDefault, FromPeerSys, IdentityKeys, InboxRequest, Payload, Relay, RemoteCloseRequest,
        RemoteConnectRequest, RemoteGetRequest, RemoteListRequest, RemoteListenRequest,
        RemoteSendRequest, RemoteSetRequest, RemoteUnsetRequest, ToPeerSys,
    },
    remotes::{REMOTES_FILE, edit_remotes},
    system::{PeerHandle, PeerSysConfig, PeerSysParams, PeerSysPaths, PeerSystem},
};
