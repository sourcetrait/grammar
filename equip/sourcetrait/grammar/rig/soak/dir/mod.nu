# A directory of Liquid templates rendered into dst with fill's values.
#
# Each `<name>.liquid` renders to `<name>`, each other file is copied, and
# directories are mirrored; `<src>/liquid/` holds the partials. An output in
# `dst` is overwritten and every other file there stays.
export def main [data: record<src: directory, dst: directory, fill: record<>>]: nothing -> nothing {
    $data.fill | soak dir $data.src $data.dst
}
