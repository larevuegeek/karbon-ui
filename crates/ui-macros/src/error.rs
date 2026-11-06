//! Diagnostics : une erreur de compilation porte son message et le span à souligner ;
//! elle devient un appel `compile_error!` à la place de la sortie de la macro.
use proc_macro::{Delimiter, Group, Literal, Span, TokenStream, TokenTree};

pub(crate) struct Error {
    pub(crate) message: String,
    pub(crate) span: Span,
}
impl Error {
    pub(crate) fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
    pub(crate) fn tokens(self) -> TokenStream {
        let mut literal = Literal::string(&self.message);
        literal.set_span(self.span);
        let mut group = Group::new(
            Delimiter::Brace,
            TokenStream::from(TokenTree::Literal(literal)),
        );
        group.set_span(self.span);
        [
            TokenTree::Ident(proc_macro::Ident::new("compile_error", self.span)),
            TokenTree::Punct(proc_macro::Punct::new('!', proc_macro::Spacing::Alone)),
            TokenTree::Group(group),
        ]
        .into_iter()
        .collect()
    }
}
pub(crate) type Result<T> = std::result::Result<T, Error>;
