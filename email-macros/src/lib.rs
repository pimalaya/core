use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(BackendContext)]
pub fn derive_backend_context(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let ident = &input.ident;

    let output = quote! {
        impl email::backend::context::BackendContext for #ident {}
    };

    TokenStream::from(output)
}

// TODO
// #[proc_macro_derive(EmailBackendContext, attributes(context))]
// pub fn derive_email_backend_context(input: TokenStream) -> TokenStream {
//     use proc_macro::TokenStream;
//     use quote::quote;
//     use syn::{parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Meta, PathSegment};

//     let input: DeriveInput = parse_macro_input!(input);

//     let mut output = quote!();

//     match input.data {
//         Data::Struct(DataStruct {
//             fields: Fields::Named(ref fields),
//             ..
//         }) => {
//             for field in &fields.named {
//                 if let Some(_ident) = &field.ident {
//                     for attr in &field.attrs {
//                         if let Attribute {
//                             meta: Meta::Path(path),
//                             ..
//                         } = attr
//                         {
//                             for segment in &path.segments {
//                                 match segment {
//                                     PathSegment { ident, .. } if ident.to_string() == "context" => {
//                                         output = quote! {
//                                         #output

//                                         impl BackendContextMapper<ImapContextSync> for MyStaticContext {
//                                             fn map_context(&self) -> Option<&ImapContextSync> {
//                                                 Some(&self.imap)
//                                             }
//                                         }

//                                                                             }
//                                     }
//                                     _ => (),
//                                 }
//                             }
//                         };
//                     }
//                 }
//             }
//         }
//         _ => (),
//     };

//     TokenStream::from(output)
// }
