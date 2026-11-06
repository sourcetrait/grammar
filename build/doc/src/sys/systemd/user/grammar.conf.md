# grammar.conf

The runtime cleanup's rule, which the user's systemd-tmpfiles reads from its
`user-tmpfiles.d/` once `install.bash`, beside it, links it there, run once by
hand after an install. A link rather than a copy, so a package's update of the
rule takes effect with the install. `%t` is the user manager's runtime
directory, `XDG_RUNTIME_DIR`, where every host keeps its tree under `grammar/`
and an agent its scratch under `ai/`.

## The install
No systemctl command places a tmpfiles rule: the rule is a file the user's
systemd-tmpfiles finds in its search path, and the clean timer is systemd's
own, `systemd-tmpfiles-clean.timer`, which systemd's user preset enables, its
`90-systemd.preset` reading `enable systemd-tmpfiles-clean.timer` on Fedora.
Where a distribution's preset leaves it off, `systemctl --user enable --now
systemd-tmpfiles-clean.timer` once turns it on.

## The lines
An `e` line clears a directory that exists by age and creates nothing: the
host makes `grammar/` itself at its start, 0700, and an agent its `ai/`, so a
`d` line, which would make them, has no place here. Three days clears a host
long gone while leaving every session's files: a session lasts hours, and what
sits there is read once. A host running longer than three days can lose a file
it still holds, as age alone decides. systemd's own
`systemd-tmpfiles-clean.timer` runs the clean, a few minutes after the user
manager starts and daily after.
