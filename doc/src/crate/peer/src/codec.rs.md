# codec.rs

## enum RemoteStream
Which of a link's two connections a handshake announces: Message (control) or
File. Carried in the initiator's Hello so the acceptor knows which connection it
just took, keeping file bulk off the control path. A Copy fieldless tag, with the
bitcode and serde derives the languages carry.

## enum MsgFrame
Peer-neutral: either end sends it. Nothing rides back - the sender's own transport
write is the delivery's ack, and remote/channel/Sent and Unsent fire from that write.

### variant Deliver
`files` names the dests the receiver awaits on the file connection before it
relays, empty for a message with none.

## enum InitiatorToAcceptor
## enum AcceptorToInitiator
Two languages, one per direction, so the type system forbids sending a message
the wrong way: a mis-send is a compile error rather than a runtime surprise. The
application frames, `Msg` and `File`, are peer-neutral and shared both ways. Named
initiator and acceptor rather than client and server: both ends are Grammar MCP
peers and the roles are per connection (a host initiates the links it opens and
accepts the links opened to it). bitcode's Encode and Decode are the wire format;
the serde derives ride along for any non-wire use. Four variants each: Hello (the
opening McpNom handshake - the initiator's also names the stream, the acceptor's
just replies its nom), Msg(MsgFrame), File(FileFrame), and Close. There is no
request/response pair: MsgFrame carries only Deliver, with no DeliverAck, because
the sender's own transport write is the delivery ack.

## const REMOTE_ZSTD_LEVEL
Every frame is zstd-compressed before framing, `zstd(bitcode(msg))`, applied
uniformly to both streams rather than gated on the stream: worth it for file chunks,
and for the control messages too, which are string-heavy (McpNoms, the dests and
events of a send) and compress well. Level 3, zstd's default, is the speed and ratio
balance; a single level keeps the codec branch-free. A tiny handshake frame pays a few
bytes of zstd overhead, accepted for the uniform rule.

## const REMOTE_INFLATED_MAX
8 MiB, which a delivery or a file chunk of 256 KiB fits well within: a frame
inflating past it is refused.

## fn remote_frame_max
The length-delimited ceiling on the wire, zstd's compress bound for a payload
of `REMOTE_INFLATED_MAX`, a few tens of KiB past it: an incompressible frame
grows a little under zstd, so a frame within the inflated cap always fits the
wire, and the sender's frame codec never refuses what the check passed.

## fn check_outbound
A frame's peer-side checks run before it is queued: its bitcode encoding
within `REMOTE_INFLATED_MAX` and a budgeted decode of it within
`DECODE_BUDGET_DEFAULT`, exactly what the receiving codec applies after it
inflates the frame. The decode costs a copy of the frame, beside the
compression that dominates a send.

## struct BitcodeCodec
A tokio-util Decoder and Encoder wrapping LengthDelimitedCodec: the length prefix frames
the stream, and each frame is zstd-compressed bitcode (encode: bitcode, then zstd, then
the frame; decode reverses it). `PhantomData<T>` binds the codec to one message type, so
a `FramedRead<_, BitcodeCodec<AcceptorToInitiator>>` can only yield that language.
io::Error is what the Framed traits want. A peer's frame is inflated within
`REMOTE_INFLATED_MAX`, so a few hundred bytes of zeros can't inflate without
bound, and decoded within the protocol's default decode budget, as a remote
engine's frames are. The round trip and both refusals are unit-tested
(`tests/link.rs`), and the system tests' remote links run it end to end.

## fn inflate
zstd's stream decoder read through a `take` one byte past the cap, so the
inflation stops there and the excess is seen.
