# client.rs

Codex's app-server over its control socket: a WebSocket on a Unix stream, its
JSON-RPC written and read here alone. Unix only, the socket being a Unix
socket.

## struct ToolOutput
What a delivery hands a thread as a tool's output: its name, its namespace, and
its text, the packet as NUON, which the model reads as `grammar.channel`.

## struct DeadlineStream
Each socket read takes only the time left before its deadline, so a frame read
slowly a chunk at a time can't outlast the bound: tungstenite's frame reader
loops over socket reads, each of which a fixed timeout would let take the whole
bound again.

## fn AppServerClient::connect
The handshake and `initialize` each run within the bound, the client naming
itself and asking the experimental API that `turn/start`'s tool output needs,
then `initialized`, which takes no answer.

## fn AppServerClient::turn_start
A turn on the thread with no input and the packet as a tool's output, which
wakes an idle thread with the packet in its model's context.

## fn AppServerClient::request
One request and its answer by its id, frames for other ids and notifications
passed by; an `error` answer is the request's refusal, and no answer within the
bound is a timeout of its own.

## fn AppServerClient::close
Best effort: the app-server ends its side on a close and sends none back, so
nothing waits for one.
