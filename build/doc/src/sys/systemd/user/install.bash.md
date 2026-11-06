# install.bash

The runtime cleanup's install, `bash <home>/sys/systemd/user/install.bash`, run
once by hand as the user it serves after a Linux install: one command, linking
`grammar.conf` beside it into `~/.config/user-tmpfiles.d/`, the directory made
where absent. The rule's path comes from the script's own, resolved, so the
link names the home it was run from and a package's update of the rule takes
effect with the install; a later run from another home replaces the link.

Nothing else is installed: systemd's user preset enables
`systemd-tmpfiles-clean.timer`, which runs the rule, and no systemctl command
places a tmpfiles rule (`grammar.conf.md`).
