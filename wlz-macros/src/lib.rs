use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, GenericArgument, Ident, Path, PathArguments, Type,
};
use heck::ToSnakeCase;

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

#[proc_macro_derive(PtrWrapper)]
pub fn derive_ptr_wrapper(input: TokenStream) -> TokenStream {
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
                let ptr = self.0.as_ptr();
                unsafe { &*self.0.as_ptr() }
            }

            pub fn as_ref_mut<'a>(&mut self) -> &'a mut #inner_ty {
                unsafe { &mut *self.0.as_ptr() }
            }
        }

        impl ::core::convert::Into<*mut #inner_ty> for &#name {
            fn into(self) -> *mut #inner_ty {
                self.0.as_ptr()
            }
        }

        impl ::core::convert::TryFrom<*mut #inner_ty> for #name {
            type Error = ();

            fn try_from(value: *mut #inner_ty) -> Result<Self, Self::Error> {
                NonNull::new(value).map(Self).ok_or(())
            }
        }
    }
    .into()
}

#[proc_macro_derive(WlListeners, attributes(listener))]
pub fn derive_wl_listeners(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;

    let fields = match input.data {
        Data::Struct(ref s) => s.fields.clone(),
        _ => {
            return syn::Error::new_spanned(&input, "WlListeners only works on structs")
                .into_compile_error()
                .into()
        }
    };

    let mut trampolines = Vec::new();
    let mut inits = Vec::new();

    let struct_snake_case = struct_name.to_string().to_snake_case();

    for field in fields {
        let field_name = field.ident.unwrap();

        for attr in field.attrs {
            if attr.path().is_ident("listener") {
                let attr = attr.parse_args::<syn::LitStr>().unwrap();
                let cb_ident = Ident::new(&attr.value(), attr.span());

                let trampoline_name = Ident::new(
                    &format!("__{}_{}_trampoline", struct_snake_case, field_name),
                    field_name.span(),
                );

                trampolines.push(quote! {
                    unsafe extern "C" fn #trampoline_name(
                        listener: *mut crate::ffi::wl_listener,
                        data: *mut std::ffi::c_void,
                    ) {
                        let this = (listener as *mut u8)
                            .sub(::memoffset::offset_of!(#struct_name, #field_name))
                            as *mut #struct_name;

                        match std::ptr::NonNull::new(listener) {
                            Some(ptr) => {
                                (*this).#cb_ident(crate::wrapper::wl::Listener::from_ptr(ptr), data);
                            },
                            None => {
                                crate::error!("failed listener callback, listener is NULL!");
                            },
                        }
                    }
                });

                let init_name = Ident::new(
                    &format!("init_{}", field_name),
                    field_name.span()
                );

                inits.push(quote! {
                    fn #init_name(signal: &mut crate::wrapper::wl::Signal) -> crate::wrapper::wl::Listener {
                        let mut listener = crate::ffi::wl_listener {
                            link: unsafe { std::mem::zeroed() },
                            notify: Some(Self::#trampoline_name),
                        };

                        let sig_ptr = signal as *mut crate::wrapper::wl::Signal;

                        unsafe {
                            crate::ffi::wl_signal_add(
                                sig_ptr as *mut crate::ffi::wl_signal,
                                &mut listener as *mut crate::ffi::wl_listener
                            );
                        }

                        crate::wrapper::wl::Listener(listener)
                    }
                });
            }
        }
    }

    quote! {
        impl #struct_name {
            #(#trampolines)*

            #(#inits)*
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn cdrop(attr: TokenStream, item: TokenStream) -> TokenStream {
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
