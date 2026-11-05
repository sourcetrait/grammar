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
MCP, nushell, their plugins, and the grammar and gram equips.

It requires gram's repository cloned beside this one, as `gram`: `ext/gram`
links to it, and the package carries its equip. It packs with `optzst`, OptZst's
own command, which must be on `PATH`, signing with the OpenPGP secret key whose file the
repository's git config names as `optzst.key`.

Installing
--------------------------------------------------------------------------------
A package is `grammar_<version>_<os>_<arch>.opt.tar.zst`. `optzst`, a static command
on every platform, checks it and places it, given a key you trust:

```bash
optzst install --place uenv-usr-local --keyring <key> grammar_<version>_<os>_<arch>.opt.tar.zst
```

`--place uenv-usr-local` and `--place uenv-usr-adhoc` install into
`$UENV_USR_LOCAL/opt/grammar` or `$UENV_USR_ADHOC/opt/grammar` and link each of
its `bin/` entries from that home's `bin/`; `--place dir --dir <path>` installs
into a directory of your own and links nothing.

- Linux: a musl system takes the `linux-musl` package.
- Windows: an elevated session for a UENV place, its links being symlinks.
  grammar's binaries need the
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
