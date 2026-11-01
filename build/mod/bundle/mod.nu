# grammar's release: cargo's builds, nushell, and the package's tree.

# The MCP's package: its binary is the release's, its version the release's.
const MCP_CRATE = 'sourcetrait_grammar_mcp'

# The worker's package, whose `grammar-ssh` the tree carries beside the MCP.
const SSH_CRATE = 'sourcetrait_grammar_ssh'

# The file-name prefix of a plugin binary, the one the MCP registers by.
const PLUGIN_PREFIX = 'nu_plugin_'

# nushell's core plugins, pulled from crates.io at the engine's version.
const CORE_PLUGINS = [
    'nu_plugin_formats'
    'nu_plugin_gstat'
    'nu_plugin_inc'
    'nu_plugin_polars'
    'nu_plugin_query'
]

# The MCP's skills the package carries, from `agent/skill/`.
const SKILLS = ['nu' 'grammar-mcp' 'grammar-mcp-dev']

# The builtins' equips the package carries: grammar's own, gram's through `ext/gram`.
const EQUIPS = [
    [name source];
    [grammar [equip sourcetrait grammar]]
    [gram [ext gram equip sourcetrait gram]]
]

# Refuse a build without gram's equip, which `ext/gram` links from gram's repository.
export def require_gram [root: path]: nothing -> nothing {
    let dir: path = ($root | path join 'ext' 'gram' 'equip' 'sourcetrait' 'gram')
    if not ($dir | path exists) {
        error make {
            msg: $"no gram equip at ($dir): gram's repository must sit beside this one, as `gram`"
        }
    }
}

# Tally every dependency's license into `LICENSES-EXTERNAL.txt` at the root, by cargo-license.
export def licenses [root: path]: nothing -> nothing {
    (^cargo license --color never --manifest-path ($root | path join 'Cargo.toml')
        | save --force ($root | path join 'LICENSES-EXTERNAL.txt'))
}

# Build the MCP, the worker, and the plugins in cargo's `release` or `debug` profile.
export def compile [root: path, profile: string]: nothing -> nothing {
    let flags: list<string> = (profile_flags $profile '--release')
    let crates: list<string> = ([$MCP_CRATE $SSH_CRATE] | append (plugin_targets $root | get package | uniq))
    let packages: list<string> = ($crates | each {|crate| ['-p' $crate] } | flatten)
    ^cargo build --locked --manifest-path ($root | path join 'Cargo.toml') ...$flags ...$packages
}

# Install nushell and its core plugins at the engine's version from crates.io.
#
# Returns the install's root, `nu/<profile>` in cargo's target directory, its
# binaries in `bin/`. Cargo skips a crate already installed there at that
# version.
export def nushell [root: path, profile: string]: nothing -> path {
    let version: string = (engine_version $root)
    let target: path = (target_dir $root)
    let nu_root: path = ($target | path join 'nu' $profile)
    let flags: list<string> = (if $profile == 'debug' { ['--debug'] } else { [] })
    let crates: list<string> = (['nu'] | append $CORE_PLUGINS | each {|crate| $"($crate)@($version)" })
    (^cargo install --locked --root $nu_root
        --target-dir ($target | path join 'nu' 'build') ...$flags ...$crates)
    $nu_root
}

# Cargo's target directory for the workspace, `CARGO_TARGET_DIR` where it is set.
export def target_dir [root: path]: nothing -> path {
    cargo_metadata $root | get target_directory
}

# The engine's nushell version: `nu-protocol`'s in the workspace lock.
export def engine_version [root: path]: nothing -> string {
    (open --raw ($root | path join 'Cargo.lock')
        | from toml | get package | where name == 'nu-protocol' | get version.0)
}

# The release's semver, the MCP crate's version, `+dbg` for `debug`.
export def package_semver [root: path, profile: string]: nothing -> string {
    let version: string = (
        cargo_metadata $root | get packages | where name == $MCP_CRATE | get version.0
    )
    if $profile == 'debug' { $"($version)+dbg" } else { $version }
}

# Assemble the package's tree in `pkg/<profile>` in cargo's target directory; returns it.
#
# `bin/` holds the MCP and the worker, `nu/bin/` nushell, `plugin/core/`
# nushell's core plugins from `nu_root`, the install nushell made,
# `plugin/grammar/` the workspace's own, `equip/` the builtins' sources, and
# `mcp/skill/` the MCP's skills. Each binary takes `os`'s executable suffix.
export def layout [root: path, profile: string, nu_root: path, os: string]: nothing -> path {
    let exe: string = (exe_suffix $os)
    let target: path = (target_dir $root)
    let built: path = ($target | path join $profile)
    let tree: path = ($target | path join 'pkg' $profile)
    if ($tree | path exists) {
        rm --recursive $tree
    }
    let bin: path = ($tree | path join 'bin')
    mkdir $bin
    for product in ['grammar-mcp' 'grammar-ssh'] {
        cp ($built | path join $"($product)($exe)") ($bin | path join $"($product)($exe)")
    }
    let nu_bin: path = ($tree | path join 'nu' 'bin')
    mkdir $nu_bin
    cp ($nu_root | path join 'bin' $"nu($exe)") ($nu_bin | path join $"nu($exe)")
    let core: path = ($tree | path join 'plugin' 'core')
    mkdir $core
    for plugin in $CORE_PLUGINS {
        cp ($nu_root | path join 'bin' $"($plugin)($exe)") ($core | path join $"($plugin)($exe)")
    }
    let grammar: path = ($tree | path join 'plugin' 'grammar')
    mkdir $grammar
    for plugin in (plugin_targets $root | get bin) {
        cp ($built | path join $"($plugin)($exe)") ($grammar | path join $"($plugin)($exe)")
    }
    for equip in $EQUIPS {
        let dest: path = ($tree | path join 'equip' 'sourcetrait' $equip.name)
        mkdir ($dest | path dirname)
        cp --recursive ($root | path join ...$equip.source) $dest
    }
    for skill in $SKILLS {
        let dest: path = ($tree | path join 'mcp' 'skill' $skill)
        mkdir $dest
        cp ($root | path join 'agent' 'skill' $skill 'SKILL.md') $dest
    }
    if $os != 'windows' {
        ^chmod -R 'u=rwX,go=rX' $tree
    }
    $tree
}

# An os's executable suffix: `.exe` on Windows, none elsewhere.
export def exe_suffix [os: string]: nothing -> string {
    if $os == 'windows' { '.exe' } else { '' }
}

# cargo's flags for a profile: `flag` for `release`, none for `debug`.
def profile_flags [profile: string, flag: string]: nothing -> list<string> {
    match $profile {
        'release' => [$flag],
        'debug' => [],
        _ => { error make {msg: $"no cargo profile `($profile)`: release or debug"} },
    }
}

# Each workspace binary named as a plugin, with its package, by cargo's metadata.
def plugin_targets [root: path]: nothing -> table<package: string, bin: string> {
    (cargo_metadata $root | get packages
        | each {|package|
            $package.targets
                | where {|target| ('bin' in $target.kind) and ($target.name | str starts-with $PLUGIN_PREFIX) }
                | each {|target| {package: $package.name, bin: $target.name} }
        }
        | flatten
        | sort-by bin)
}

# cargo's metadata for the workspace's own packages.
def cargo_metadata [root: path]: nothing -> record {
    (^cargo metadata --manifest-path ($root | path join 'Cargo.toml') --no-deps
        --format-version 1 | from json)
}
