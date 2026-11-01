# A repository's tree with sizes, leaving out .git and what .gitignore ignores.
#
# Outside a repository no .gitignore applies.
export def main [data: record<dir: directory>]: nothing -> record<tree: string> {
    { tree: (eye git $data.dir) }
}
