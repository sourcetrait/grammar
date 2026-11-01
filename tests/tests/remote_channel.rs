//! RemoteChannel's runtime proof: two real hosts linked over mTLS.
//!
//! Each host has a simulated rmcp consumer, stdio JSON-RPC and a real WSS
//! Channel client, proving the transport end to end: the Channel's connect,
//! verify, and receive, the A->B->A echo, the Connected and Disconnected
//! notices, the synchronous listen and connect, each remote's state in
//! `interface remote channel list`, the remotes file read and edited live, and the
//! oversized event's spill. Keys are one-off and trusted directly, so this
//! needs no OS trust store and runs unattended.
//!
//! The remote channels are the MCP's own: a host here is the server, its bare
//! entry, and a definition on its engine drives the cases that send.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use sourcetrait_grammar_tests::*;
use sourcetrait_common::testing::prelude::*;

static TESTING: testing::Module = testing::module!(Integration, { .using_temp_dir() });

/// The reusable consumer, standalone: connect a real WSS client to a host's
/// Channel against a test-minted CA, verify, and receive a definition's packet.
#[tested]
fn consumer_connects_verifies_and_receives_a_packet() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();

    let ch = generate_keys("grammar", &root.join("channel"));
    write_host_config(&root.join("host"), &ch, "");
    let host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "chan", "--namespace", "default"],
        &root.join("host"),
    );
    let mut c = Consumer::attach(host, &ch.ca);

    // A definition's grimm channel send must arrive on the real WSS client, and the
    // record it returns, bound by a `let`, must name the packet that arrived.
    let resp = c.host.execute(
        "def execute [data: nothing]: nothing -> record<event_id: string> {\n    let sent = (grimm channel send 'test/Ping' {msg: 'hello-wss'})\n    {event_id: $sent.event_id}\n}",
        "null",
    );
    assert!(!has_error_path(&resp), "the channel send failed: {resp}");
    let event_id = envelope(&resp)["result"]["event_id"]
        .as_str()
        .expect("the send returns {event_id}")
        .to_string();

    let packet = c
        .await_packet(|p| p.model == "test/Ping", Duration::from_secs(10))
        .expect("consumer should receive the test/Ping over WSS");
    assert_eq!(
        packet.id, event_id,
        "the returned event_id is the id of the packet that arrived; got {packet:?}",
    );
    assert!(
        packet.event.contains("hello-wss"),
        "the packet event should carry the payload; got {packet:?}",
    );
    assert_eq!(
        packet.from.split('/').next(),
        Some("thread"),
        "a definition-origin packet is from `thread/<nonce>`; got {}",
        packet.from,
    );
}

/// A host whose config names no `[identity]` refuses to open the Channel, naming
/// the table it lacks.
#[tested]
fn a_channel_without_an_identity_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let cwd = root.join("bare");
    std::fs::create_dir_all(&cwd).expect("mkdir the start directory");
    let mut host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "bare", "--namespace", "default"],
        &cwd,
    );
    let open = host.interface("interface channel open async");
    assert!(has_error_path(&open), "the Channel presents an identity it lacks; got {open}");
    assert_eq!(err_kind(&open), "channel::start", "the refusal's kind; got {open}");
    assert!(
        open.to_string().contains("[identity]"),
        "the refusal names the table it lacks; got {open}",
    );
}

/// `--config` names the file in place of the start directory's own, which then
/// goes unread: a broken one there would fail the start.
#[tested]
fn the_config_flag_replaces_the_start_directorys_file() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let ch = generate_keys("grammar", &root.join("channel"));
    let named = write_host_config(&root.join("named"), &ch, "");
    let cwd = root.join("host");
    std::fs::create_dir_all(cwd.join(".grammar")).expect("mkdir .grammar");
    std::fs::write(cwd.join(".grammar").join("config.toml"), "not [toml")
        .expect("write a broken config");
    let host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "flag", "--namespace", "default", "--config", named.to_str().unwrap()],
        &cwd,
    );
    Consumer::attach(host, &ch.ca);
}

/// The full transport proof plus the ConnectionWoes contract: a synchronous
/// blocking open (listener on bind, connector on connect), the A->B->A echo over
/// a real mTLS link, and Disconnected on teardown for both roles.
#[tested]
fn remote_loopback_echo() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (mut a, mut b, nom_a, nom_b) = spawn_two_hosts(root, Host::spawn_in);

    // The listener (B) opens - blocks on bind, returns synchronously once bound.
    // The connector (A) then opens - blocks on connect (B is already bound),
    // returns synchronously once paired. Neither open is void-then-async.
    pair(&mut a, &mut b, &nom_a);
    // The connector gets NO Connected for its open attempt (the synchronous open
    // success is the notice); a stray one within a grace window is a failure.
    assert!(
        a.await_packet(|p| p.model == "remote/channel/Connected", Duration::from_secs(2)).is_none(),
        "the connector open attempt emits no Connected",
    );

    // A sends test/Ping to B; A sees its own remote/channel/Sent (driven by the write).
    let send_id = remote_send(&mut a, &nom_b, "test/Ping", "note: \"ping-1\"");
    a.await_packet(
        |p| p.model == "remote/channel/Sent" && p.event.contains(&send_id),
        Duration::from_secs(15),
    )
    .expect("A should see its own Sent for the ping");

    // B's agent observes the relayed packet on B's Channel...
    let relayed = b
        .await_packet(
            |p| p.model == "test/Ping" && p.from == format!("remote/channel/{nom_a}"),
            Duration::from_secs(15),
        )
        .expect("B should receive the relayed test/Ping");
    assert!(relayed.event.contains("ping-1"), "relayed event carries the payload; got {relayed:?}");

    // ...and echoes it back to A as test/Loopback - the return leg.
    remote_send(&mut b, &nom_a, "test/Loopback", "echo: \"ping-1\"");
    let looped = a
        .await_packet(
            |p| p.model == "test/Loopback" && p.from == format!("remote/channel/{nom_b}"),
            Duration::from_secs(15),
        )
        .expect("A should receive the test/Loopback echo - the round-trip");
    assert!(looped.event.contains("ping-1"), "the loopback carries the original payload; got {looped:?}");

    // Teardown: closing the link on A drops the established link; Disconnected is
    // retained for an established-link teardown on BOTH roles.
    let close = a.host.interface("interface remote channel close 'peer'");
    assert!(!has_error_path(&close), "`interface remote channel close` succeeds on an open link; got {close}");
    a.await_packet(|p| p.model == "remote/channel/Disconnected", Duration::from_secs(15))
        .expect("the connector sees Disconnected on teardown");
    b.await_packet(|p| p.model == "remote/channel/Disconnected", Duration::from_secs(15))
        .expect("the listener sees Disconnected on teardown");
}

/// A connect whose dial is refused fails synchronously, `remote::channel::open_failed`.
///
/// No Disconnected follows for the attempt. The 20 s connect timeout on a peer
/// that accepts and hangs is the code's constant, not forced here: it would
/// cost 20 s a run.
#[tested]
fn a_refused_connect_fails_synchronously() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();

    let cert_a = generate_keys("a", &root.join("cert_a"));
    let cert_dead = generate_keys("dead", &root.join("cert_dead"));

    // A free (unbound) port - a dial there is refused immediately (not a timeout).
    let dead = format!("127.0.0.1:{}", free_port());
    let host_dir = root.join("host");
    write_host_config(&host_dir, &cert_a, &remote_connector(&dead, &cert_dead.public_key));
    let host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "dc", "--namespace", "default"],
        &host_dir,
    );
    let mut c = Consumer::attach(host, &cert_a.ca);

    let open = c.host.interface("interface remote channel connect 'peer'");
    assert!(has_error_path(&open), "a refused connect fails synchronously; got {open}");
    assert_eq!(err_kind(&open), "remote::channel::open_failed", "the sync failure kind; got {open}");
    assert!(
        c.await_packet(|p| p.model == "remote/channel/Disconnected", Duration::from_secs(3)).is_none(),
        "the connect attempt emits no async Disconnected",
    );
}

/// A second listen on a listening remote is refused, `remote::channel::already_open`.
#[tested]
fn a_second_listen_is_refused() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();

    let cert_b = generate_keys("b", &root.join("cert_b"));
    let cert_peer = generate_keys("peer", &root.join("cert_peer"));

    let listen = format!("127.0.0.1:{}", free_port());
    let host_dir = root.join("host");
    write_host_config(&host_dir, &cert_b, &remote_listener(&listen, &cert_peer.public_key));
    let host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "dl", "--namespace", "default"],
        &host_dir,
    );
    let mut c = Consumer::attach(host, &cert_b.ca);

    let first = c.host.interface("interface remote channel listen 'peer'");
    assert!(!has_error_path(&first), "the first listen binds synchronously; got {first}");
    let second = c.host.interface("interface remote channel listen 'peer'");
    assert!(has_error_path(&second), "a second listen is refused; got {second}");
    assert_eq!(err_kind(&second), "remote::channel::already_open", "the refusal's kind; got {second}");
}

/// Each remote's row reads its state: `closed`, then `listening` at its
/// endpoint, then `open` with its peer's nom once a connect pairs.
#[tested]
fn remote_list_shows_each_remotes_state() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (mut a, mut b, nom_a, nom_b) = spawn_two_hosts(root, Host::spawn_in);

    let closed = remote_row(&mut b, "peer");
    assert_eq!(closed["state"], "closed", "got {closed}");
    assert!(closed["endpoint"].is_nothing(), "no endpoint while closed; got {closed}");

    let listened = b.host.interface("interface remote channel listen 'peer'");
    assert!(!has_error_path(&listened), "the listen is synchronous on its bind; got {listened}");
    let endpoint = envelope(&listened)["result"]["endpoint"].clone();
    let listening = remote_row(&mut b, "peer");
    assert_eq!(listening["state"], "listening", "got {listening}");
    assert_eq!(listening["endpoint"], endpoint, "the endpoint the listen bound; got {listening}");

    let connected = a.host.interface("interface remote channel connect 'peer'");
    assert!(!has_error_path(&connected), "the connect is synchronous; got {connected}");
    b.await_packet(
        |p| p.model == "remote/channel/Connected" && p.event.contains(&nom_a),
        Duration::from_secs(15),
    )
    .expect("B sees Connected on pairing");
    let open = remote_row(&mut b, "peer");
    assert_eq!(open["state"], "open", "got {open}");
    assert_eq!(open["remote_mcp_nom"], nom_a, "the peer's nom; got {open}");
    let dialed = remote_row(&mut a, "peer");
    assert_eq!(dialed["state"], "open", "got {dialed}");
    assert_eq!(dialed["remote_mcp_nom"], nom_b, "the peer's nom; got {dialed}");
}

/// The remotes file is read and edited at each command, with no restart.
///
/// A set makes and changes an entry, keeping the file's comments and its other
/// entries; a get and a list read it; an unset removes it; a malformed file
/// refuses the command, naming itself.
#[tested]
fn remote_entries_are_set_read_and_unset_live() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let keys = generate_keys("live", &root.join("cert"));
    let peer = generate_keys("peer", &root.join("cert_peer"));
    let host_dir = root.join("host");
    write_host_config(&host_dir, &keys, "");
    let mut host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "live", "--namespace", "default"],
        &host_dir,
    );
    let listed = host.interface("interface remote channel list");
    assert_eq!(envelope(&listed)["result"], nuon("[]"), "no remotes yet; got {listed}");

    let remotes = host_dir.join(".grammar").join("remotes.toml");
    std::fs::write(&remotes, "# kept\n[[channel]]\nalias = \"kept\"\ncert = \"/k/kept.pem\"\n")
        .expect("an entry written by hand");
    let cert = peer.public_key.to_str().expect("a UTF-8 path");
    let set = host.interface(&format!(
        "interface remote channel set 'peer' --cert '{cert}' --address '127.0.0.1:9'"
    ));
    assert!(!has_error_path(&set), "the set holds; got {set}");
    assert_eq!(envelope(&set)["result"]["cert"], cert, "it answers the row; got {set}");
    let text = std::fs::read_to_string(&remotes).expect("the file");
    assert!(text.starts_with("# kept\n"), "the file's comment stays; got {text}");
    assert!(text.contains("alias = \"kept\""), "and its other entry; got {text}");

    let changed = host.interface("interface remote channel set 'peer' --listen '127.0.0.1'");
    assert!(!has_error_path(&changed), "a change holds; got {changed}");
    let row = remote_row_of(&mut host, "peer");
    assert_eq!(row["listen"], "127.0.0.1", "the field changed; got {row}");
    assert_eq!(row["address"], "127.0.0.1:9", "the rest as it was; got {row}");
    assert_eq!(row["state"], "closed", "got {row}");

    let refused = host.interface("interface remote channel set 'other' --address '127.0.0.1:9'");
    assert_eq!(err_kind(&refused), "remote::channel::invalid_params", "a new remote needs its cert");

    let unset = host.interface("interface remote channel unset 'peer'");
    assert!(!has_error_path(&unset), "the unset holds; got {unset}");
    let gone = host.interface("interface remote channel get 'peer'");
    assert_eq!(err_kind(&gone), "remote::channel::invalid_params", "got {gone}");
    let aliases: Vec<NuVal> = envelope(&host.interface("interface remote channel list"))["result"]
        .as_list()
        .expect("a table")
        .iter()
        .map(|row| row["alias"].clone())
        .collect();
    assert_eq!(aliases, vec!["kept"], "the other entry stays");

    std::fs::write(&remotes, "not [toml").expect("a broken file");
    let broken = host.interface("interface remote channel list");
    assert_eq!(err_kind(&broken), "remote::file", "got {broken}");
    assert!(broken.to_string().contains("remotes.toml"), "it names the file; got {broken}");
}

/// With no listen address, a listen binds loopback on a random port, and a
/// connect dials the endpoint it is given, entries made after the start.
#[tested]
fn a_listen_defaults_to_loopback_and_a_connect_takes_its_endpoint() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let cert_a = generate_keys("a", &root.join("cert_a"));
    let cert_b = generate_keys("b", &root.join("cert_b"));
    let mut b = attach_bare(root, "b", &cert_b);
    let mut a = attach_bare(root, "a", &cert_a);
    let nom_a = mcp_nom(&mut a);
    for (c, peer) in [(&mut b, &cert_a), (&mut a, &cert_b)] {
        let cert = peer.public_key.to_str().expect("a UTF-8 path");
        let set = c.host.interface(&format!("interface remote channel set 'peer' --cert '{cert}'"));
        assert!(!has_error_path(&set), "the set holds; got {set}");
    }

    let listened = b.host.interface("interface remote channel listen 'peer'");
    assert!(!has_error_path(&listened), "got {listened}");
    let endpoint = envelope(&listened)["result"]["endpoint"]
        .as_str()
        .expect("the listen answers its endpoint")
        .to_string();
    let bound: std::net::SocketAddr = endpoint.parse().expect("an ip:port");
    assert_eq!(bound.ip().to_string(), "127.0.0.1", "loopback by default; got {endpoint}");
    assert_ne!(bound.port(), 0, "a random port, bound; got {endpoint}");

    let unaddressed = a.host.interface("interface remote channel connect 'peer'");
    assert_eq!(err_kind(&unaddressed), "remote::channel::invalid_params", "no address to dial");
    let connected = a
        .host
        .interface(&format!("interface remote channel connect 'peer' --address '{endpoint}'"));
    assert!(!has_error_path(&connected), "the connect takes its endpoint; got {connected}");
    b.await_packet(
        |p| p.model == "remote/channel/Connected" && p.event.contains(&nom_a),
        Duration::from_secs(15),
    )
    .expect("B sees Connected on pairing");
}

/// A listen on a taken port fails, and one on its ip alone binds a random port.
#[tested]
fn a_taken_port_fails_and_its_ip_alone_listens_on_a_random_one() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let cert_b = generate_keys("b", &root.join("cert_b"));
    let cert_peer = generate_keys("peer", &root.join("cert_peer"));
    let taken = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("a port to take");
    let port = taken.local_addr().expect("its address").port();
    let host_dir = root.join("host");
    let listen = format!("127.0.0.1:{port}");
    write_host_config(&host_dir, &cert_b, &remote_listener(&listen, &cert_peer.public_key));
    let host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "taken", "--namespace", "default"],
        &host_dir,
    );
    let mut c = Consumer::attach(host, &cert_b.ca);

    let clash = c.host.interface("interface remote channel listen 'peer'");
    assert_eq!(err_kind(&clash), "remote::channel::open_failed", "the port is taken; got {clash}");
    let retried = c.host.interface("interface remote channel listen 'peer' --on '127.0.0.1'");
    assert!(!has_error_path(&retried), "a listen on the ip alone binds; got {retried}");
    let endpoint: std::net::SocketAddr = envelope(&retried)["result"]["endpoint"]
        .as_str()
        .expect("its endpoint")
        .parse()
        .expect("an ip:port");
    assert_ne!(endpoint.port(), port, "not the taken port");
    assert_ne!(endpoint.port(), 0, "a port bound");
    drop(taken);
}

/// CapNoCap (local): an oversized channel_send event auto-spills to an inbox file;
/// a compact pointer rides the wire under the notification cap, and the full event
/// survives in the spill file.
#[tested]
fn channel_send_spills_oversized_event() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();

    let ch = generate_keys("grammar", &root.join("channel"));
    write_host_config(&root.join("host"), &ch, "");
    let host = Host::spawn_in(
        &root.join("scratch"),
        &["--id", "spill", "--namespace", "default"],
        &root.join("host"),
    );
    let mut c = Consumer::attach(host, &ch.ca);

    let resp = c.host.execute(
        "def execute [data: nothing]: nothing -> record<ok: bool> {\n    grimm channel send 'test/Big' {marker: 'SPILL_MARKER_42', blob: (1..3000 | each { 'x' } | str join)}\n    {ok: true}\n}",
        "null",
    );
    assert!(!has_error_path(&resp), "the channel send failed: {resp}");

    let pkt = c
        .await_packet(|p| p.model == "test/Big", Duration::from_secs(10))
        .expect("consumer should receive the test/Big packet");
    assert!(
        pkt.event.contains("spilled_event_path"),
        "an oversized event is replaced by a spill pointer; got {pkt:?}",
    );
    assert!(
        !pkt.event.contains("SPILL_MARKER_42"),
        "the event content is spilled out of the wire packet; got {pkt:?}",
    );
    assert!(
        pkt.raw.chars().count() < 500,
        "the pointer packet stays under the notification cap; got {} chars",
        pkt.raw.chars().count(),
    );

    let content = c.read_inbox(&spill_path(&pkt));
    assert!(
        content.contains("SPILL_MARKER_42"),
        "the full event survives in the spill file; got {content}",
    );
}

/// CapNoCap (remote): an oversized remote_channel_send event auto-spills via the
/// file connection; the relayed packet on the receiver is a pointer under the cap,
/// and the full event lands in the receiver's inbox before the packet is seen.
#[tested]
fn remote_send_spills_oversized_event() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (mut a, mut b, nom_a, nom_b) = spawn_two_hosts(root, Host::spawn_in);
    pair(&mut a, &mut b, &nom_a);

    let big = "marker: \"REMOTE_SPILL_99\", blob: (1..3000 | each { \"x\" } | str join)";
    remote_send(&mut a, &nom_b, "test/BigRemote", big);

    let relayed = b
        .await_packet(
            |p| p.model == "test/BigRemote" && p.from == format!("remote/channel/{nom_a}"),
            Duration::from_secs(15),
        )
        .expect("B should receive the relayed oversized event");
    assert!(
        relayed.event.contains("spilled_event_path"),
        "the relayed oversized event is a spill pointer; got {relayed:?}",
    );
    assert!(
        !relayed.event.contains("REMOTE_SPILL_99"),
        "the event content is spilled out of the wire packet; got {relayed:?}",
    );
    assert!(
        relayed.raw.chars().count() < 500,
        "the relayed pointer stays under the notification cap; got {} chars",
        relayed.raw.chars().count(),
    );
    assert!(relayed.attached.is_some(), "the relayed packet names the delivery dir; got {relayed:?}");

    let content = b.read_inbox(&spill_path(&relayed));
    assert!(
        content.contains("REMOTE_SPILL_99"),
        "the full event survives in B's inbox spill file; got {content}",
    );
}

/// `grimm remote_channel_send_with` returns `{event_id}`, and that id is the one
/// the sender's own Sent notice carries; the files reach the receiver beside the
/// relayed packet.
#[tested]
fn remote_send_with_returns_its_event_id() {
    let t = testing::test!({ .using_temp_dir() });
    let root = t.temp_dir();
    let (mut a, mut b, nom_a, nom_b) = spawn_two_hosts(root, Host::spawn_in);
    pair(&mut a, &mut b, &nom_a);

    let src = root.join("note.txt");
    std::fs::write(&src, "files-1").expect("write the attachment");
    let resp = a.host.execute(
        "def execute [data: record<nom: string, src: string>]: nothing -> record<id: string> {\n    let sent = (grimm remote_channel_send_with $data.nom 'test/Files' {note: 'files-1'} [[src dest]; [$data.src 'note.txt']])\n    {id: $sent.event_id}\n}",
        &NuVal::Record(vec![
            ("nom".to_string(), NuVal::from(nom_b.to_string())),
            ("src".to_string(), NuVal::from(src.to_str().expect("utf-8 path"))),
        ])
        .to_string(),
    );
    assert!(!has_error_path(&resp), "remote_channel_send_with failed: {resp}");
    let send_id = envelope(&resp)["result"]["id"]
        .as_str()
        .expect("the send returns {event_id}")
        .to_string();

    a.await_packet(
        |p| p.model == "remote/channel/Sent" && p.event.contains(&send_id),
        Duration::from_secs(15),
    )
    .expect("A's own Sent carries the returned event_id");
    let relayed = b
        .await_packet(
            |p| p.model == "test/Files" && p.from == format!("remote/channel/{nom_a}"),
            Duration::from_secs(15),
        )
        .expect("B should receive the relayed test/Files");
    assert!(relayed.event.contains("files-1"), "the relayed event carries the payload; got {relayed:?}");
    assert!(relayed.attached.is_some(), "the relayed packet names its delivery dir; got {relayed:?}");
}

// ---- helpers ----

/// The host's nom, as its Channel's Open packet carried it.
fn mcp_nom(c: &mut Consumer) -> String {
    c.nom.clone()
}

/// Drive `grimm remote channel send` from a definition and return the message id.
fn remote_send(c: &mut Consumer, peer_nom: &str, model: &str, event_body: &str) -> String {
    let definition = format!(
        "def execute [data: record<nom: string>]: nothing -> record<id: string> {{\n    let sent = (grimm remote channel send $data.nom '{model}' {{{event_body}}})\n    {{id: $sent.event_id}}\n}}",
    );
    let data = NuVal::Record(vec![("nom".to_string(), NuVal::from(peer_nom.to_string()))]);
    let resp = c.host.execute(&definition, &data.to_string());
    assert!(!has_error_path(&resp), "the remote channel send failed: {resp}");
    envelope(&resp)["result"]["id"].as_str().expect("send returns a message id").to_string()
}

/// The first error row's `kind` from an error envelope.
fn err_kind(resp: &Value) -> String {
    envelope_error_kind(resp).unwrap_or_default()
}

/// One remote's row, as `interface remote channel get` answers it.
fn remote_row(c: &mut Consumer, alias: &str) -> NuVal {
    remote_row_of(&mut c.host, alias)
}

/// One remote's row from a host with no consumer.
fn remote_row_of(host: &mut Host, alias: &str) -> NuVal {
    let got = host.interface(&format!("interface remote channel get '{alias}'"));
    assert!(!has_error_path(&got), "the get holds; got {got}");
    envelope(&got)["result"].clone()
}

/// A host named `name` with its identity alone, its consumer attached.
fn attach_bare(root: &Path, name: &str, keys: &GeneratedKeys) -> Consumer {
    let dir = root.join(format!("host_{name}"));
    write_host_config(&dir, keys, "");
    let host = Host::spawn_in(
        &root.join(format!("scratch_{name}")),
        &["--id", &format!("bare_{name}"), "--namespace", "default"],
        &dir,
    );
    Consumer::attach(host, &keys.ca)
}

/// The inbox-relative spill path from a spill-pointer packet's event.
fn spill_path(pkt: &Packet) -> String {
    let ev = NuVal::from_nuon(&pkt.event).expect("event parses as NUON");
    ev["spilled_event_path"].as_str().expect("a spill pointer carries spilled_event_path").to_string()
}

/// Spawn a listener host (B) and a connector host (A) cross-configured on one free
/// loopback port, each with an attached consumer, each host made by `spawn`.
/// Returns (a, b, nom_a, nom_b); no link is opened yet.
fn spawn_two_hosts(
    root: &Path,
    spawn: fn(&Path, &[&str], &Path) -> Host,
) -> (Consumer, Consumer, String, String) {
    let cert_a = generate_keys("a", &root.join("cert_a"));
    let cert_b = generate_keys("b", &root.join("cert_b"));
    let listen = format!("127.0.0.1:{}", free_port());

    let b_dir = root.join("host_b");
    write_host_config(&b_dir, &cert_b, &remote_listener(&listen, &cert_a.public_key));
    let host_b = spawn(
        &root.join("scratch_b"),
        &["--id", "link_b", "--namespace", "default"],
        &b_dir,
    );
    let mut b = Consumer::attach(host_b, &cert_b.ca);

    let a_dir = root.join("host_a");
    write_host_config(&a_dir, &cert_a, &remote_connector(&listen, &cert_b.public_key));
    let host_a = spawn(
        &root.join("scratch_a"),
        &["--id", "link_a", "--namespace", "default"],
        &a_dir,
    );
    let mut a = Consumer::attach(host_a, &cert_a.ca);

    let nom_a = mcp_nom(&mut a);
    let nom_b = mcp_nom(&mut b);
    (a, b, nom_a, nom_b)
}

/// B listens and A connects, both synchronous, then B's Connected is awaited, so
/// the link is established when this returns.
fn pair(a: &mut Consumer, b: &mut Consumer, nom_a: &str) {
    let open_b = b.host.interface("interface remote channel listen 'peer'");
    assert!(!has_error_path(&open_b), "the listen is synchronous on its bind; got {open_b}");
    let open_a = a.host.interface("interface remote channel connect 'peer'");
    assert!(!has_error_path(&open_a), "the connect is synchronous; got {open_a}");
    b.await_packet(
        |p| p.model == "remote/channel/Connected" && p.event.contains(nom_a),
        Duration::from_secs(15),
    )
    .expect("B sees Connected on pairing");
}
