# verify.rs

## struct PublicKeyVerifier
The security core of the remote transport. Trust is KNOWN-PUBLIC-KEY, not CA-chain:
a peer is accepted iff its presented end-entity leaf DER is byte-equal to the one
configured known public key. Deliberately independent of the system trust store and
the CA validation the WSS channel uses - a remote link authenticates a SPECIFIC peer
host (its srcert-minted entity key), not "anyone our CA signed". The nouns are
PublicKey and PrivateKey; never pin, entity, union, leaf, or cert in our own names.

One PublicKeyVerifier implements BOTH rustls verifier traits because each end
matches the SAME remote public key regardless of role: as the connecting client it
is the ServerCertVerifier (checks the remote's server cert), as the accepting server
it is the ClientCertVerifier (checks the remote's client cert). mTLS runs both, so
both traits are needed and both match identically. A listener matches the one
configured peer of its link through a single-leaf `server_config`, never a union over
every configured peer.

### the signature methods are not optional
The match is necessary but not sufficient: matching the leaf DER proves it is the
right key, but the handshake SIGNATURE proves the peer holds the matching private
key (a public leaf alone can be replayed). So verify_tls12/13_signature delegate to
the ring provider's signature_verification_algorithms rather than being stubbed -
stubbing them is the classic "comes up working and unauthenticated" hole.
supported_verify_schemes reports the provider's real schemes so rustls negotiates
one the delegation can check.

### root_hint_subjects empty
The hint tells a client which CAs the server trusts - meaningless under
known-public-key (there is no CA to hint). Empty tells the client to send whatever
leaf it has, which the match then checks.

The provider is ring, matching the crate's other TLS; no second backend. The unit
test (`tests/link.rs`) covers the match decision over raw DER blobs; the
full handshake is the system tests'.
