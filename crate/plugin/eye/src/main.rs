/// Serves the eye plugin to its engine over stdio, in MessagePack.
fn main() {
    nu_plugin::serve_plugin(&nu_plugin_eye::EyePlugin, nu_plugin::MsgPackSerializer {});
}
