# home.rs

grammar-ssh resolves its homes itself, in the MCP's order, and its data and
cache homes are the MCP's own roots on every platform, so grammar-ssh and the
MCP find one namespace's files in one place. The MCP reads them from its
process's variables as it goes (the MCP's `home.rs`); grammar-ssh takes its
environment as data and reports what it ignored, so a test resolves homes
without touching the real ones.

## const DATA, const CACHE
Each is its UENV variable, then XDG's, then the platform's folder, the order the
MCP reads its two roots in on every platform. The folder is what `directories`
answers the MCP: XDG's default under `HOME` on Linux; `Library/Application
Support` and `Library/Caches` under `HOME` on macOS; and on Windows the roaming
and the local application data folders, which grammar-ssh reads as their
variables, `APPDATA` and `LOCALAPPDATA`, each a session's own. `directories`
reads XDG's variables on Linux alone, so grammar-ssh and the MCP each read them
first themselves. `tests/home.rs` holds grammar-ssh to the MCP's order under one
environment, XDG's variables set and unset.

## const TMP
On Windows the temporary home is `TMP`, then `TEMP`, the order Windows' own
`GetTempPath2W` reads them in, where Unix's is `TMPDIR`, then `/tmp`.

## enum Default
`UnderHome` and `Absolute` serve Unix's defaults, `Var` Windows' variables read
last, `TEMP` and the two application data folders; each platform keeps only the
kinds it uses.

## struct Homes
### fn resolve
Each home is its UENV variable, then the standard one behind it, then its
default: `/tmp` or `TEMP` for the temporary home, and the data and cache homes
as above. The runtime home has no default, since a guessed one could sit where
another user writes; without one the transient scratch falls back beside the
bulk one (`scratch.rs`).

### field diagnostics
Each variable ignored for being empty or relative, which the start prints as
the MCP prints its own: a misconfiguration shows rather than quietly moving
files.

## fn resolve_home
A chosen base is validated, never made: a home that does not exist is refused
by name, since making a platform's home on a guess could put grammar-ssh's
files where the user's tools never look.
