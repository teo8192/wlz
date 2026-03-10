use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, FnArg, GenericArgument, Ident, Item, LitStr, Pat, Path, PathArguments, Receiver, ReturnType, Token, Type, parse::{Parse, ParseStream}, parse_macro_input, spanned::Spanned
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

            /// # Safety
            /// This creates a dangling pointer to the object, MUST BE INITIALIZED LATER!
            pub unsafe fn empty() -> #name {
                Self(::std::ptr::NonNull::dangling())
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
                ::std::ptr::NonNull::new(value).map(Self).ok_or(())
            }
        }
    }
    .into()
}

struct ListenerAttr {
    name: LitStr,
    ty: Option<Path>
}

impl Parse for ListenerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // parse the first argument (string literal)
        let name: LitStr = input.parse()?;

        // check if there is a comma + second argument
        let ty = if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            Some(input.parse::<Path>()?)
        } else {
            None
        };

        Ok(Self { name, ty })
    }
}

#[proc_macro_derive(WlListeners, attributes(listener))]
pub fn derive_wl_listeners(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = &input.ident;
    let generics = &input.generics;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
        let field_name = &field.ident.unwrap();

        let field_ty = &field.ty;

        for attr in field.attrs.iter().filter(|a| a.path().is_ident("listener")) {
            let listener = attr.parse_args::<ListenerAttr>().unwrap();
            let cb_ident = Ident::new(&listener.name.value(), listener.name.span());

            // check if field type is listener
            if match field_ty {
                Type::Path(path) => !path.path.is_ident("Listener"),
                _ => true,
            } {
                return syn::Error::new_spanned(field_ty, "listener attribute are only for fields of Listener type")
                    .into_compile_error()
                    .into();
            }

            let trampoline_name = Ident::new(
                &format!("__{}_{}_trampoline", struct_snake_case, field_name),
                field_name.span(),
            );

            let func_call = if let Some(ty_path) = &listener.ty {
                quote! {
                    let data_mut_ref = &mut (*(data as *mut #ty_path));
                    (*this).#cb_ident(data_mut_ref)
                }
            } else {
                quote! {
                    (*this).#cb_ident()
                }
            };

            trampolines.push(quote! {
                /// Trampoline for to be used in C callbacks.
                unsafe extern "C" fn #trampoline_name(
                    listener: *mut crate::ffi::wl_listener,
                    data: *mut std::ffi::c_void,
                ) {
                    if listener.is_null() {
                        crate::error!("failed listener callback, listener is NULL!");
                        return;
                    }

                    let this = (listener as *mut u8)
                        .sub(::memoffset::offset_of!(#struct_name, #field_name))
                        as *mut #struct_name;

                    #func_call;
                }
            });

            let init_name = Ident::new(
                &format!("init_{}", field_name),
                field_name.span()
            );

            inits.push(quote! {
                /// In place initialization
                fn #init_name(&mut self) {
                    self.#field_name.init(Self::#trampoline_name);
                }
            });
        }
    }

    quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            #(#trampolines)*

            #(#inits)*
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn c_drop(attr: TokenStream, item: TokenStream) -> TokenStream {
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

#[proc_macro_attribute]
pub fn c_ptr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let c_type = parse_macro_input!(attr as Path);
    let input = parse_macro_input!(item as DeriveInput);

    let name = &input.ident;

    if !has_repr_c(&input.attrs) {
        return syn::Error::new_spanned(&input, "c_ptr can only be used with repr(\"C\")").into_compile_error().into();
    }

    quote! {
        #input

        impl #name {
            /// # Safety
            /// the pointer must be valid for mutable access
            pub unsafe fn from_ptr<'a>(ptr: ::std::ptr::NonNull<#c_type>) -> &'a mut #name {
                let ptr = ptr.as_ptr() as *mut #name;
                &mut *ptr
            }

            pub fn as_ptr(&mut self) -> *mut #c_type {
                (self as *mut #name) as *mut #c_type
            }
        }
    }
    .into()
}


#[proc_macro_derive(FromPtr)]
pub fn from_ptr(input: TokenStream) -> TokenStream {
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
                &mut *ptr
            }

            /// Get a pointer to the first element
            pub fn as_ptr(&mut self) -> *mut #field_ty {
                (&mut self.0) as *mut #field_ty
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn initialization(attr: TokenStream, item: TokenStream) -> TokenStream {
    // ensute the attribute has no arguments
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "initialization takes no arguments")
            .to_compile_error()
            .into();
    }

    let item = parse_macro_input!(item as Item);

    let func = match &item {
        Item::Fn(f) => f,
        _ => {
            return syn::Error::new(item.span(), "initialization can only be used on functions")
                .to_compile_error()
                .into();
        }
    };

    let vis = &func.vis;
    let sig = &func.sig;
    let fn_name = &sig.ident;

    // make sure &mut self
    match sig.inputs.first() {
        Some(FnArg::Receiver(Receiver { reference: Some(_), mutability: Some(_), .. })) => {},
        Some(first) => {
            return syn::Error::new(first.span(), "expected &mut self")
                .to_compile_error()
                .into();
        }
        None => {
            return syn::Error::new(sig.span(), "expected &mut self")
                .to_compile_error()
                .into();
        }
    }

    let args: Vec<_> = sig.inputs.iter().skip(1).collect();
    let arg_idents: Vec<_> = args.iter().map(|arg| {
        match arg {
            FnArg::Typed(pat) => {
                if let Pat::Ident(id) = &*pat.pat {
                    &id.ident
                } else {
                    panic!("unsupported pattern")
                }
            }
            _ => unreachable!()
        }
    }).collect();

    let ret = &sig.output;

    enum ReturnKind {
        Result,
        Option,
        Plain,
        Unknown
    }

    fn get_return_type(ty: &ReturnType) -> &Type {
        match ty {
            ReturnType::Type(_, ty) => &**ty,
            _ => panic!("unexpected type")
        }
    }

    fn classify_return(ty: &ReturnType) -> ReturnKind {
        match ty {
            ReturnType::Type(_, ty) => {
                if let Type::Path(p) = &**ty {
                    let seg = &p.path.segments.last().unwrap().ident;
                    if seg == "Result" {
                        ReturnKind::Result
                    } else if seg == "Option" {
                        ReturnKind::Option
                    } else {
                        ReturnKind::Unknown
                    }
                } else {
                    ReturnKind::Unknown
                }
            },
            ReturnType::Default => ReturnKind::Plain,
        }
    }

    fn extract_result(ty: &Type) -> Option<(Type, Type)> {
        if let Type::Path(type_path) = ty {
            let segment = type_path.path.segments.last()?;

            if segment.ident == "Result" && let PathArguments::AngleBracketed(args) = &segment.arguments {
                let mut iter = args.args.iter();
                let ok = match iter.next()? {
                    GenericArgument::Type(t) => t.clone(),
                    _ => return None, 
                };

                let err = match iter.next()? {
                    GenericArgument::Type(t) => t.clone(),
                    _ => return None,
                };

                return Some((ok, err));
            }
        }

        None
    }

    let ret_kind = classify_return(ret);

    let ret_type = match ret_kind {
        ReturnKind::Result => {
            let (_ok, err) = extract_result(get_return_type(ret)).unwrap();
            // TODO make sure ok is ()
            quote! { Result<::std::pin::Pin<Box<Self>>, #err> }
        },
        ReturnKind::Option => quote! { Option<::std::pin::Pin<Box<Self>>> },
        ReturnKind::Plain => quote! { ::std::pin::Pin<Box<Self>> },
        ReturnKind::Unknown => {
            return syn::Error::new(ret.span(), "must either be Result<(), T>, Option<()> or ()")
                .to_compile_error()
                .into();
        },
    };

    let result_handler = match ret_kind {
        ReturnKind::Result |
        ReturnKind::Option => quote! {
            result.map(|_| new_box)
        },
        ReturnKind::Plain => quote! { new_box },
        ReturnKind::Unknown => unreachable!(),
    };

    quote! {
        #func

        /// In place initialization
        #vis fn initialize(mut uninit: ::std::pin::Pin<Box<::std::mem::MaybeUninit<Self>>> #(, #args)*) -> #ret_type {
            let box_ptr = unsafe { uninit.as_mut().get_unchecked_mut().as_mut_ptr() };
            ::std::mem::forget(uninit);

            let result = unsafe { box_ptr.as_mut().unwrap() }.#fn_name(#(#arg_idents),*);

            let new_box = Box::into_pin(unsafe { Box::from_raw(box_ptr) });

            #result_handler
        }
    }
    .into()
}

