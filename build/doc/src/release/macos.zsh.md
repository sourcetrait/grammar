# macos.zsh

macOS's tar carries extended attributes and AppleDouble files by default, which
would land in the package's tree: `--no-mac-metadata` and `COPYFILE_DISABLE=1`
keep the tar to the tree's own files.
