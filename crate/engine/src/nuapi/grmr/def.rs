use crate::*;

const SPAN: nu::Span = nu::Span::unknown();

impl GrmrChannelSend {
    pub(crate) const DEF: nuin::SignatureDef<GrmrCategory> = nuin::SignatureDef {
        name: "grmr channel send",
        description: "Emits a message to the Channel. Returns a message id on success.",
        category: GrmrCategory::Tool,
        examples: &[
            nuin::ExampleDef {
                description: "Send arbitrary event data (as seen from: nu(execute))",
                example: "grmr channel send 'my/adhoc/Thing' {foo:'bar', num:3}",
                result_fn: || nu::Value::record(
                    nu::record! {
                        "event_id" => nu::Value::string("987654321X", nu::Span::unknown()),
                    },
                    nu::Span::unknown()
                ),
            },
            nuin::ExampleDef {
                description: "Send arbitrary event data (as seen from: Channel)",
                example: "grmr channel send 'my/adhoc/Thing' {foo:'bar', num:3}",
                result_fn: || nu::Value::record(
                    nu::record! {
                        "id" => nu::Value::string("987654321X", SPAN),
                        "from" => nu::Value::string("thread/0123456789A", SPAN),
                        "model" => nu::Value::string("my/adhoc/Thing", SPAN),
                        "data" => nu::Value::record(
                            nu::record! {
                                "foo" => nu::Value::string("bar", SPAN),
                                "num" => nu::Value::int(3, SPAN),
                            },
                            SPAN
                        ),
                    },
                    SPAN
                ),
            },
        ],
    };
    pub(crate) const DEF_MODEL: nuin::ParameterDef = nuin::ParameterDef {
        name: "model",
        description: "Namepath of the data's shape",
    };
    pub(crate) const DEF_DATA: nuin::ParameterDef = nuin::ParameterDef {
        name: "data",
        description: "Notification data",
    };
    pub(crate) const DEF_ATTACHED: nuin::ParameterDef = nuin::ParameterDef {
        name: "attached",
        description: "Detailed data. Stored in the Channel inbox for retrieval",
    };
}

impl GrmrRemoteChannelSend {
    pub(crate) const DEF: nuin::SignatureDef<GrmrCategory> = nuin::SignatureDef {
        name: "grmr remote channel send",
        description: "Emits a message to a remote Channel. Returns a message id on success.",
        category: GrmrCategory::Tool,
        examples: &[
            nuin::ExampleDef {
                description: "Send arbitrary data to a remote",
                example: "grmr remote channel send '0123456789B' 'my/model/Event' {foo:'bar', num:3}",
                result_fn: || nu::Value::record(nu::record!{
                    "event_id" => nu::Value::string("0987654321Z", nu::Span::unknown()),
                }, nu::Span::unknown()),
            },
            nuin::ExampleDef {
                description: "Send data to a remote with a file, received as report.nuon",
                example: "grmr remote channel send '0123456789B' 'my/model/Event' {foo:'bar'} [[src dest]; ['out.nuon' 'report.nuon']]",
                result_fn: || nu::Value::record(nu::record!{
                    "event_id" => nu::Value::string("0987654321Y", nu::Span::unknown()),
                }, nu::Span::unknown()),
            },
        ],
    };
}

