# A directory's tree with sizes, skipping ignore globs unless regard globs match.
#
# Globs match each path below `dir`: `**/target` skips every `target`, `target`
# the top one alone, and a trailing slash matches nothing. `.git` is skipped
# unless regarded.
export def main [data: record<dir: directory, ignore: list<glob>, regard: list<glob>>]: nothing -> record<tree: string> {
    { tree: (eye dir $data.dir $data.ignore $data.regard) }
}
