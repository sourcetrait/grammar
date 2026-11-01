use crate::*;

/// The kind of a nushell `NetworkError`, a DNS failure by its own kind.
#[cereal::derived(Data, Copy, Eq, Hash)]
pub enum NetworkErrorKind {
    Generic,
    DnsAgain,
    DnsNoName,
    DnsNoData,
    DnsFail,
}

impl NetworkErrorKind {
    /// The diagnostic code nushell gives the kind.
    pub const fn code(self) -> Option<&'static str> {
        Some(match self {
            Self::Generic => "nu::shell::network",
            Self::DnsAgain => "nu::shell::network::dns::again",
            Self::DnsNoName => "nu::shell::network::dns::no_name",
            Self::DnsNoData => "nu::shell::network::dns::no_data",
            Self::DnsFail => "nu::shell::network::dns::fail",
        })
    }

    /// The kind's message, the same for every error of the kind.
    pub const fn message(self) -> &'static str {
        match self {
            Self::Generic => "Network failure",
            Self::DnsAgain => "Temporary failure in name resolution",
            Self::DnsNoName => "Name or service is unknown",
            Self::DnsNoData => "Host exists but has no address records",
            Self::DnsFail => "Non recoverable failure in name resolution",
        }
    }
}

#[cfg(feature = "nushell")]
impl From<&nu::NetworkError> for NetworkErrorKind {
    fn from(v: &nu::NetworkError) -> Self {
        match v {
            nu::NetworkError::Generic { .. } => Self::Generic,
            nu::NetworkError::Dns(dns) => match dns.kind {
                nu::DnsErrorKind::Again => Self::DnsAgain,
                nu::DnsErrorKind::NoName => Self::DnsNoName,
                nu::DnsErrorKind::NoData => Self::DnsNoData,
                nu::DnsErrorKind::Fail => Self::DnsFail,
            },
        }
    }
}
