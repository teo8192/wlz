use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Path, Type};

#[proc_macro_derive(PtrWrapper)]
pub fn derive_ptr_wrapper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let field_ty =
        match input.data {
            syn::Data::Struct(ref s) => match s.fields {
                syn::Fields::Unnamed(ref fields) => &fields.unnamed.first().unwrap().ty,
                _ => return syn::Error::new_spanned(
                    &input,
                    format!(
                        "{} is not a tuple struct! PtrWrapper may only be used on tuple structs",
                        name
                    ),
                )
                .into_compile_error()
                .into(),
            },
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    format!(
                        "{} is not a struct! PtrWrapper may only be used on structs",
                        name
                    ),
                )
                .into_compile_error()
                .into()
            }
        };

    let inner_ty = match field_ty {
        Type::Ptr(ptr) => &*ptr.elem,
        _ => {
            return syn::Error::new_spanned(
                &input,
                format!(
                    "{} does not have a pointer field, needed for PtrWrapper",
                    name
                ),
            )
            .into_compile_error()
            .into()
        }
    };

    let expanded = quote! {
        impl #name {
            pub fn as_ptr(&self) -> *const #inner_ty {
                self.0
            }

            pub fn as_mut_ptr(&mut self) -> *mut #inner_ty {
                self.0
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn cdrop(attr: TokenStream, item: TokenStream) -> TokenStream {
    let free_fn = parse_macro_input!(attr as Path);
    let input = parse_macro_input!(item as DeriveInput);

    let name = &input.ident;

    let expanded = quote! {
        #input

        impl Drop for #name {
            fn drop(&mut self) {
                unsafe {
                    #free_fn(self.0);
                }
            }
        }
    };

    expanded.into()
}
