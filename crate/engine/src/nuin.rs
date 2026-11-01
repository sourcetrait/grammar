use crate::*;

/// A command's signature as static data: name, summary, category, examples.
pub(crate) struct SignatureDef<CAT: SignatureCategoryTrait> {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) category: CAT,
    pub(crate) examples: &'static [ExampleDef],
}

/// One example of a command, its result built when the examples are asked.
pub(crate) struct ExampleDef {
    pub(crate) description: &'static str,
    pub(crate) example: &'static str,
    pub(crate) result_fn: fn() -> nu::Value,
}

impl<CAT: SignatureCategoryTrait> SignatureDef<CAT> {
    /// The examples as nushell's `Example`s.
    pub(crate) fn examples(&'static self) -> Vec<nu::Example<'static>> {
        self.examples.iter()
            .map(|x| nu::Example {
                description: x.description,
                example: x.example,
                result: Some((x.result_fn)()),
            })
            .collect()
    }
}

/// A command category: the root category's name and the category's own term.
pub(crate) trait SignatureCategoryTrait: Copy {
    fn root() -> &'static str;
    fn str(&'static self) -> &'static str;
}

/// A parameter's name and summary.
pub(crate) struct ParameterDef {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
}

/// A signature taking its summary, category, and search term from a def.
pub(crate) trait SignatureTrait<CAT: SignatureCategoryTrait> {
    fn vocab(self, sigdef: &'static SignatureDef<CAT>) -> Self;
}

/// A signature taking its parameters from defs.
pub(crate) trait SignatureParameterTrait {
    fn vocab_required(self, paramdef: &'static ParameterDef, shape: nu::SyntaxShape) -> Self;
    fn vocab_optional(self, paramdef: &'static ParameterDef, shape: nu::SyntaxShape) -> Self;
}

impl<CAT: SignatureCategoryTrait> SignatureTrait<CAT> for nu::Signature {
    fn vocab(self, sigdef: &'static SignatureDef<CAT>) -> Self {
        self
            .description(sigdef.description)
            .category(nu::Category::Custom(CAT::root().into()))
            .search_terms(vec![sigdef.category.str().into()])
    }
}

impl SignatureParameterTrait for nu::Signature {
    fn vocab_required(self, paramdef: &'static ParameterDef, shape: nu::SyntaxShape) -> Self {
        self.required(paramdef.name, shape, paramdef.description)
    }

    fn vocab_optional(self, paramdef: &'static ParameterDef, shape: nu::SyntaxShape) -> Self {
        self.optional(paramdef.name, shape, paramdef.description)
    }
}
