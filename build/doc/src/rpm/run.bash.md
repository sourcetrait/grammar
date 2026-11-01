# run.bash

The machine's side of the rpm's build, in bash, a Fedora machine holding its
distribution's tools alone; it takes the server build's arguments, so a runner
driving `build/container/run.bash` drives it too.

## The phases
The source rpm is assembled with the network, the one phase that needs it.
`rpmbuild --rebuild` then runs on it with `--network none`, the proof that
the source carries everything a builder without a network needs, copr's among
them. The rpm is installed on the base image itself, also with no network, and
each binary runs, so the package stands on a system that has nothing but the
distribution.

## The sources and labels
Each repository is mounted alone, read-only, with `Z` for SELinux, as the
server build mounts them.

## The cleanup
As the server build's: the EXIT trap removes the image, the base when this run
pulled it, and the work directory beside `--out`, whatever the outcome; HUP,
INT, and TERM become exits so it runs for them, and `--init` passes a signal
on to the container's command.
