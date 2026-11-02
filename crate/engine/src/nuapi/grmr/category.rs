use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub(crate) enum GrmrCategory {
    Utility,
    Tool,
    Control,
}

impl GrmrCategory {
    const ROOT: &'static str = "grmr";
    const UTILITY: &'static str = "grmr::utility";
    const TOOL: &'static str = "grmr::tool";
    const CONTROL: &'static str = "grmr::control";
}

impl nuin::SignatureCategoryTrait for GrmrCategory {
    #[inline]
    fn root() -> &'static str { Self::ROOT }
    
    #[inline]
    fn str(&'static self) -> &'static str {
        match self {
            Self::Utility => Self::UTILITY,
            Self::Tool => Self::TOOL,
            Self::Control => Self::CONTROL,
        }
    }
}
