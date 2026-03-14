use proc_macro::TokenStream;
use quote::{quote};
use syn::{
    Attribute, DeriveInput, GenericArgument, Path, PathArguments, Type, parse_macro_input
};

fn extract_nonnull_inner(ty: &Type) -> syn::Result<&Type> {
    if let Type::Path(type_path) = ty {
        let segment = type_path.path.segments.last().unwrap();

        if segment.ident != "NonNull" {
            return Err(syn::Error::new_spanned(ty, "Expected NonNull<T>"));
        }

        if let PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(GenericArgument::Type(inner)) = args.args.first() {
            return Ok(inner);
        }
    }

    Err(syn::Error::new_spanned(ty, "Expected NonNull<T>"))
}

// #[proc_macro_derive(PtrWrapper)]
pub(crate) fn derive_ptr_wrapper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let field_ty = match input.data {
        syn::Data::Struct(ref s) => match s.fields {
            syn::Fields::Unnamed(ref fields) => &fields.unnamed.first().unwrap().ty,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    format!(
                        "{} is not a tuple struct! PtrWrapper may only be used on tuple structs",
                        name
                    ),
                )
                .into_compile_error()
                .into()
            }
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

    let inner_ty = match extract_nonnull_inner(field_ty) {
        Ok(ts) => ts,
        Err(e) => {
            return e.into_compile_error().into();
        }
    };

    quote! {
        impl #name {
            pub fn as_ptr(&self) -> *mut #inner_ty {
                self.0.as_ptr()
            }

            pub fn as_ref<'a>(&self) -> &'a #inner_ty {
                unsafe { self.0.as_ptr().as_ref().unwrap() }
            }

            pub fn as_mut<'a>(&mut self) -> &'a mut #inner_ty {
                unsafe { self.0.as_ptr().as_mut().unwrap() }
            }

            /// # Safety
            /// This creates a dangling pointer to the object, MUST BE INITIALIZED LATER!
            pub unsafe fn empty() -> #name {
                Self(::std::ptr::NonNull::dangling())
            }
        }

        impl ::core::convert::From<&#name> for *mut #inner_ty {
            fn from(value: &#name) -> *mut #inner_ty {
                value.0.as_ptr()
            }
        }

        impl ::core::convert::TryFrom<*mut #inner_ty> for #name {
            type Error = ();

            fn try_from(value: *mut #inner_ty) -> Result<Self, Self::Error> {
                ::std::ptr::NonNull::new(value).map(Self).ok_or(())
            }
        }

        impl ::core::convert::From<NonNull<#inner_ty>> for #name {
            fn from(value: NonNull<#inner_ty>) -> Self {
                Self(value)
            }
        }
    }
    .into()
}

// #[proc_macro_attribute]
pub(crate) fn c_drop(attr: TokenStream, item: TokenStream) -> TokenStream {
    let free_fn = parse_macro_input!(attr as Path);
    let input = parse_macro_input!(item as DeriveInput);

    let name = &input.ident;

    quote! {
        #input

        impl Drop for #name {
            fn drop(&mut self) {
                unsafe {
                    #free_fn(self.as_ptr());
                }
            }
        }
    }
    .into()
}

fn has_repr_c(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("repr"))
        .any(|attr| {
            let mut repr_c = false;
            attr.parse_nested_meta(|meta| {
                repr_c = meta.path.is_ident("C");
                Ok(())
            }).unwrap();
            repr_c
        })
}

// #[proc_macro_attribute]
pub(crate) fn c_ptr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let c_type = parse_macro_input!(attr as Path);
    let input = parse_macro_input!(item as DeriveInput);

    let generics = &input.generics;
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    if !has_repr_c(&input.attrs) {
        return syn::Error::new_spanned(&input, "c_ptr can only be used with repr(\"C\")").into_compile_error().into();
    }

    quote! {
        #input

        impl #impl_generics #name #ty_generics #where_clause {
            /// # Safety
            /// the pointer must be valid for mutable access
            pub unsafe fn from_ptr<'a>(ptr: ::std::ptr::NonNull<#c_type>) -> &'a mut Self {
                let ptr = ptr.as_ptr() as *mut #name #ty_generics;
                ptr.as_mut().unwrap()
            }

            pub fn as_ptr(&mut self) -> *mut #c_type {
                (self as *mut Self) as *mut #c_type
            }
        }
    }
    .into()
}

// #[proc_macro_derive(FromPtr)]
pub(crate) fn from_ptr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let field_ty = match input.data {
        syn::Data::Struct(ref s) => match s.fields {
            syn::Fields::Unnamed(ref fields) => &fields.unnamed.first().unwrap().ty,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    format!(
                        "{} is not a tuple struct! FromPtr may only be used on tuple structs",
                        name
                    ),
                )
                .into_compile_error()
                .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input,
                format!(
                    "{} is not a struct! FromPtr may only be used on structs",
                    name
                ),
            )
            .into_compile_error()
            .into()
        }
    };

    quote! {
        impl #name {
            /// # Safety
            /// the pointer must be valid for mutable access
            pub unsafe fn from_ptr<'a>(ptr: ::std::ptr::NonNull<#field_ty>) -> &'a mut #name {
                let offset = ::memoffset::offset_of!(#name, 0);
                let ptr = (ptr.as_ptr() as *mut u8).wrapping_sub(offset) as *mut #name;
                ptr.as_mut().unwrap()
            }

            /// Get a pointer to the first element
            pub fn as_ptr(&mut self) -> *mut #field_ty {
                (&mut self.0) as *mut #field_ty
            }
        }
    }
    .into()
}
