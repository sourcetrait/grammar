# install.bash

The cleanup installed once by hand after a Linux install, in bash, which every
such host has; an OptZst's install links `bin/` and runs nothing of its own.

## The checks
Every check runs before the first change: the rule beside the script, a
`systemctl`, a user manager answering for this user, and its
`systemd-tmpfiles-clean.timer`. A host short of any is refused by name, so a
container or a login with no user manager learns what it lacks.

## The config home
The link goes where the user manager's systemd-tmpfiles reads, its own
`XDG_CONFIG_HOME` from `systemctl --user show-environment`, or `~/.config`
where it sets none, rather than the shell's, which the manager need not share.
The config home itself is checked, never made; `user-tmpfiles.d/` is a
drop-in directory a user has only once a rule is in it, so its absence is the
normal state and it is made.

## The link
The rule is linked, so a package's update, which replaces the home's files in
place, carries it. A link of an earlier run, one naming a `sys/systemd/`
rule, is relaid; a link elsewhere or a file of that name is someone else's and
refuses the run.

## The timer
`enable --now` is idempotent, and systemd's preset has the timer on already on
Fedora; the run makes it so wherever a preset doesn't.
