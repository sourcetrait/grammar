SourceTrait Grammar MCP
================================================================================
[![License Badge]][License] [![Rein Badge]][AI]

Harness MCP powered by Nushell.

Agent-maintained documentation is located in [./doc/ai/doc.yaml](doc/ai).

Skills are located in [./agent/skill](agent/skill).

Tested with:
- Claude Code
- ChatGPT Codex
- Mistral Vibe CLI (GLM)

Building
--------------------------------------------------------------------------------
`nu build.nu` builds the release, a signed OptZst in `target/pkg/` carrying the
MCP, nushell, their plugins, and the grammar and gram equips, with an install
script for each OS to place it: `install.bash` for Linux, `install.zsh` for
macOS, and `install.ps1` for Windows.

It requires gram's repository cloned beside this one, as `gram`: `ext/gram`
links to it, and the package carries its equip. It packs with `optzst`, OptZst's
own command, which must be on `PATH`, signing with the OpenPGP secret key whose file the
repository's git config names as `optzst.key`.

Installing
--------------------------------------------------------------------------------
A package is `grammar_<version>_<os>_<arch>.opt.tar.zst`. `optzst` checks and places
it, on any OS, given a key you trust:

```bash
optzst install --place uenv-usr-local --keyring <key> grammar_<version>_<os>_<arch>.opt.tar.zst
```

Without `optzst`, unpack it, check both signatures with a key you trust, and run its
install script beside the unpacked files:

```bash
tar -xf grammar_<version>_<os>_<arch>.opt.tar.zst
gpgv --keyring <key> grammar.tar.gpg.sig grammar.tar
gpgv --keyring <key> package-grammar.tar.gpg.sig package-grammar.tar
tar -xf package-grammar.tar
bash install.bash --place uenv-usr-local
```

`--place uenv-usr-local` and `--place uenv-usr-adhoc` install into
`$UENV_USR_LOCAL/opt/grammar` or `$UENV_USR_ADHOC/opt/grammar` and link each of
its `bin/` entries from that home's `bin/`; `--place dir --dir <path>` installs
into a directory of your own and links nothing. The script checks `grammar.tar`
against its record first.

- Linux: `install.bash` under bash 5 or later, and a tar that reads zstd, with
  `zstd` installed. A musl system takes the `linux-musl` package. Alpine needs
  `apk add bash zstd`, and its BusyBox tar reads no zstd, so the package unpacks
  by `zstd -dc <package> | tar -xf -`.
- macOS: `install.zsh` under the zsh macOS ships, as `zsh -f install.zsh`, so
  your startup files leave its environment alone. macOS's tar reads zstd only
  through the `zstd` program, so the unpack takes one on `PATH`, Homebrew's
  among them.
- Windows: `install.ps1` under PowerShell 7, in an elevated session for a UENV
  place, its links being symlinks. grammar's binaries need the
  [latest supported Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170#latest-supported-redistributable-version)
  installed first.

Repository
--------------------------------------------------------------------------------

Found a bug?  
Let us know! Upvote an existing issue or create one if not found.

Have questions, concerns, ideas, or requests?  
Upvote an existing discussion or create one if not found.

### AI
LLM generation is heavily used.

### Contributors
Contributors, please review [SOURCETRAIT.md](./SOURCETRAIT.md).  

#### Copyright Assignment Agreement (CAA)
By committing to this repository you
[agree to assign](./docs/legal/Copyright_Assignment_Agreement.md)
to [Asmov LLC](https://asmov.software)
all right, title, and interest worldwide in all copyright covering your
contribution.


License (AGPL3)
--------------------------------------------------------------------------------
SourceTrait Grammar MCP
Developed by [SourceTrait](https://sourcetrait.com), a division of **Asmov LLC**  
Copyright (C) 2026 [Asmov LLC](https://asmov.software)  

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a [copy](./LICENSE-AGPL-3.txt) of the
GNU Affero General Public License along with this program.
If not, see https://www.gnu.org/licenses/.


[Docs Badge]: https://img.shields.io/badge/docs-blue
[License]: #License-AGPL3
[License Badge]: https://img.shields.io/badge/license-AGPL3-blue.svg
[AI]: #AI
[Rein Badge]: https://img.shields.io/badge/rein-AI-blue.svg
