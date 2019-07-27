use syn::{
    parse::{Parse, ParseStream},
    Result,
};

/// Define custom keywords that alter which `struct`s the field is emitted on.
pub(crate) mod kw {
    syn::custom_keyword!(auto);
    syn::custom_keyword!(readonly);
    syn::custom_keyword!(private);
}

/// Represent the keywords as a type
pub(crate) enum Keyword {
    /// Cannot be inserted or updated
    Auto,

    /// Cannot be updated
    Readonly,

    /// Not serialized by serde
    Private,
}

impl Parse for Keyword {
    /// Allow `syn` to call our `parse` method
    /// and receive a `Keyword` back.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(kw::auto) {
            input.parse::<kw::auto>()?;
            Ok(Keyword::Auto)
        } else if input.peek(kw::readonly) {
            input.parse::<kw::readonly>()?;
            Ok(Keyword::Readonly)
        } else if input.peek(kw::private) {
            input.parse::<kw::private>()?;
            Ok(Keyword::Private)
        } else {
            Err(input.error("expected optional keyword `auto`, `readonly`, or `private`"))
        }
    }
}
