# chat_send.rs

`interface chat send`, the standard message between agents: a `channel/Chat`
to a linked host, its summary on the line and its message always attached as
`message.md`.

## fn ChannelFace::send_chat
The summary is checked before anything goes: the packet carries it whatever the
message's length, so it is one line of 80 characters at most. The chat spills
its message ahead of its data's spill: the sender renders the packet its
receiver will, `from` its own `remote/channel/<nom>` and `attached` in the
spilled form the receiver takes before it spills `data`, through `line_fits`,
and where the message inline would pass the cap it goes as `message.md`'s size
instead. So the receiver spills `attached` at most and never `data`, and the
summary always arrives. The send is the host's, `HostSendRequest`, since a body
may not stamp `channel/`; its delivery is reported as a remote send's is,
`remote/channel/Sent` or `Unsent`, and a refusal of the links' is
`remote::channel::invalid_params`.
