# run.bash

The machine's side of a server's build, in bash, since a server holds its
distribution's tools alone, nushell not among them.

## The sources
Each repository is mounted alone, read-only, rather than their parent, so the
container sees the six and nothing beside them. `ext/gram`, grammar's
relative link to gram, resolves inside the container as it does outside, gram
mounted beside grammar.

## The labels
`Z` relabels each mount for this container alone where SELinux enforces, as on
Fedora, whose policy refuses a container an unlabelled mount; where SELinux is
off podman ignores it. The tree in `<out>` keeps the container's label, so an
install copies its modes and times alone (`cp -R --preserve=mode,timestamps`),
its files taking their new home's label; `cp -a` carries the label over in the
xattrs it keeps, `--no-preserve=context` notwithstanding. A label already
carried over is a customizable type, which `restorecon` resets under `-F`
alone.

## The work directory
Beside `--out` rather than in `/tmp`, often a tmpfs, since a release build's
target runs to gigabytes.

## The cleanup
The EXIT trap runs on every exit, a failure's included, and HUP, INT, and TERM
are made exits so it runs for them too. The work directory and the image are
this run's by construction, named by its stamp; the base is removed only when
this run pulled it, so the machine's own images stay. `--init` puts catatonit
at the container's PID 1, which passes a signal podman forwards on to
build.bash and ends the container with it; a bash at PID 1 would ignore every
signal it has no handler for.
