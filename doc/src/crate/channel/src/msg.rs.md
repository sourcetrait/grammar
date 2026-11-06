# msg.rs

The Channel's requests, its answers, and what it sends up unasked. Each is the
crate's own type, wrapping the wire's where it carries wire data: a send carries
`ChannelSendData`, the state answers `ChannelState`, the thresholds
`SpamThresholdsData`.

## struct OpenRequest
The face mints the epoch, since the face starts the hub whose notices carry it;
the system takes it as the open's generation. A sync Channel's greeting comes
rendered as a `HostRequest`, since the Open packet is the face's (`hub.rs`).

### field inbox
A string, the path the face made: a message is data, and the face refuses an
inbox that is not UTF-8 before it opens.

## struct CloseRequest
### field prune
Only a deliberate `interface channel close` takes the inbox. The shutdown close,
the verify timer's expiry, and the unverified-emit teardown leave it readable,
so an attachment outlives a Channel the caller did not close.

## struct ExpireRequest
Carries its epoch, so a timer armed for an earlier open closes nothing.

## enum HubNotice
What the face's hub knows first: its first claimant, and its peer's end. A
notice for another epoch is stale and changes nothing.

## struct ReadyRequest
Answers the epoch it verified, so the face stands down that open's verify timer
alone.

## struct HostRequest
The greeting, which the face renders and the gate never holds. Its id and the
whole packet's NUON ride beside the line, so the system writes `<id>.nuon`
before the line goes out, as it does for every other packet.

## struct AttachedFile
A file's path in `<id>/` and its size, the row of a packet's `attached` table.
The size is the sender's to give, a body's attachment's length or a peer's
landed file's, so the Channel stats nothing on its loop.

## struct EmitRequest
A host packet through the gate - an emergency's announcement, a remote link's
relay or report - its event data, rendered and its id minted by the system on
the Channel's one counter. A relay's files arrive staged, and the system moves
them under the id it mints, so the Channel's directory has one owner.

## struct Staged
Inbox-relative, since a message carries data rather than a path of the host's.
`spill` names the peer's spilled event among the files, which the packet's
`<id>.nuon` takes in rather than a file in `<id>/`.

## enum EmitError
### variant Write
The moves and writes go before the line, so a failed one refuses the emit, and
no packet goes out whose files aren't there.

## struct SendRequest
A body's send as data. The eval's stop stays on the sender's side, since a
closure never crosses.

## struct StoppedNotice
The stop a sender made. The action is the sender's account of what it did,
known only after the stop verdict, so the error's emergency rises once the
sender reports it.

## enum ChannelSendError
Distinguishable on purpose: a background consumer must tell "stop working, the
channel is gone" from "this one send failed", or it will either exit on a blip
or spin against a dead channel.

## enum SendVerdict
`Stop` is a refusal that asks the sender to stop its own eval, as the host's
`stop` closure did.

## struct SpamStop
### field notify
True only on an episode's first crossing: the refusal persists for every later
send, but announcing each would make the report about spam into spam itself.

### field reason
The refusal the sender answers with, the policy's words, so they read the same
from any sender.

## struct Line / struct CloseHub
Each carries its epoch, so a face's pump delivers a line, or signals a frame, to
the hub of the open it concerns and drops one for any other.

## macro request_of / macro answer_of
Each request names its answer's type, and each answer comes back out of
`FromChannelSys` by its variant, as subsys pairs a request with its response.
Two requests, a close and an expiry, share `Closed`.
