use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Field, Fields, FnArg, GenericArgument, Ident, Item, Pat, Path, PathArguments, ReturnType, Token, Type, TypePath, parse::{Parse, ParseStream}, parse_macro_input, spanned::Spanned
};
use heck::ToSnakeCase;

struct Callback {
    callback: Path,
}

impl Parse for Callback {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "callback" {
            return Err(syn::Error::new(key.span(), "expected `callback`"));
        }

        input.parse::<Token![=]>()?;

        let callback: Path = input.parse()?;

        Ok(Self { callback })
    }
}

fn listener_arg_type(ty: &Type) -> syn::Result<Option<Type>>  {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Err(syn::Error::new_spanned(ty,"expected Listener<T>"))
    };

    let segment = path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new_spanned(ty, "invalid type"))?;

    if segment.ident != "Listener" {
        return Err(syn::Error::new_spanned(&segment.ident, "expected Listener"));
    }

    match &segment.arguments {
        PathArguments::None => {
            // Listener -> default to ()
            Ok(None)
        }
        PathArguments::AngleBracketed(args) => {
            if let Some(GenericArgument::Type(ty)) = args.args.first() {
                Ok(Some(ty.clone()))
            } else {
                Err(syn::Error::new_spanned(args, "expected Listener<T>"))
            }
        }
        _ => Err(syn::Error::new_spanned(segment, "unsupported Listener type")),
    }
}

fn get_listeners(fields: Fields) -> syn::Result<Vec<(Field, Callback, Option<Type>)>> {
    let mut res = Vec::new();

    for field in fields {
        if let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("listener")) {
            let listener = attr.parse_args::<Callback>()?;
            let arg_type = listener_arg_type(&field.ty)?;
            res.push((field, listener, arg_type));
        }
    }

    Ok(res)
}

fn trampoline_name(field_name: &Ident, struct_name: &Ident) -> Ident {
    Ident::new(
        &format!("__{}_{}_trampoline", struct_name.to_string().to_snake_case(), field_name),
        field_name.span(),
    )
}

fn create_trampoline(struct_name: &Ident, field_name: &Ident, listener: &Callback, arg_type: &Option<Type>) -> impl ToTokens + use<> {
    let cb_ident = listener.callback.get_ident().unwrap();

    let trampoline_name = trampoline_name(field_name, struct_name);

    let func_call = match arg_type {
        Some(ty) => quote! {
            #cb_ident(::std::pin::Pin::new_unchecked((data as *mut #ty).as_mut().unwrap()))
        },
        None => quote! {
            #cb_ident()
        },
    };

    quote! {
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

            unsafe { ::std::pin::Pin::new_unchecked(this.as_mut().unwrap()) }.#func_call;
        }
    }
}

fn init_name(field_name: &Ident) -> Ident {
    Ident::new(
        &format!("__init_{}", field_name),
        field_name.span()
    )
}

fn create_init(struct_name: &Ident, field_name: &Ident) -> impl ToTokens + use<> {
    let trampoline_name = trampoline_name(field_name, struct_name);

    let init_name = init_name(field_name);

    quote! {
        /// In place initialization
        fn #init_name(mut self: ::std::pin::Pin<&mut Self>) {
            self.project().#field_name.init(Self::#trampoline_name);
        }
    }
}

// #[proc_macro_derive(WlListeners, attributes(listener))]
pub(crate) fn derive_wl_listeners(input: TokenStream) -> TokenStream {
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
    let mut init_calls = Vec::new();

    let listeners = match get_listeners(fields) {
        Ok(v) => v,
        Err(e) => return e.into_compile_error().into()
    };

    for (field, listener, arg_type) in listeners {
        let field_name = &field.ident.unwrap();

        trampolines.push(create_trampoline(struct_name, field_name, &listener, &arg_type));
        init_calls.push(init_name(field_name));
        inits.push(create_init(struct_name, field_name));
    }

    quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            #(#trampolines)*

            #(#inits)*

            fn __initialize_callbacks(mut self: ::std::pin::Pin<&mut Self>) {
                #(self.as_mut().#init_calls();)*
            }
        }
    }
    .into()
}

// #[proc_macro_attribute]
pub(crate) fn initialization(attr: TokenStream, item: TokenStream) -> TokenStream {
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
    // TODO: Better check possibly? The initializer accepts a lot of shit now i guess
    /*match sig.inputs.first() {
        Some(FnArg::Receiver(Receiver {
            reference: None,
            colon_token: Some(_),
            ty: _ty,
            ..
        })) => {
            // TODO: check that ty is &mut Pin<&mut self>
        },
        _ => {
            return syn::Error::new(sig.span(), "expected self: &mut Pin<&mut Self>")
                .to_compile_error()
                .into();
        }
    }*/

    let args: Vec<_> = sig.inputs.iter().skip(1).collect();
    let arg_idents: Vec<_> = args.iter().map(|arg| {
        match arg {
            FnArg::Typed(pat) => {
                if let Pat::Ident(id) = pat.pat.as_ref() {
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
            ReturnType::Type(_, ty) => ty,
            _ => panic!("unexpected type")
        }
    }

    fn classify_return(ty: &ReturnType) -> ReturnKind {
        match ty {
            ReturnType::Type(_, ty) => {
                if let Type::Path(p) = ty.as_ref() {
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

    let lifetime_name = quote! {'__lifetime};

    let ret_type = match ret_kind {
        ReturnKind::Result => {
            let (_ok, err) = extract_result(get_return_type(ret)).unwrap();
            // TODO make sure ok is ()
            quote! { Result<::std::pin::Pin<&#lifetime_name mut Self>, #err> }
        },
        ReturnKind::Option => quote! { Option<::std::pin::Pin<&#lifetime_name mut Self>> },
        ReturnKind::Plain => quote! { ::std::pin::Pin<&#lifetime_name mut Self> },
        ReturnKind::Unknown => {
            return syn::Error::new(ret.span(), "must either be Result<(), T>, Option<()> or ()")
                .to_compile_error()
                .into();
        },
    };

    let result_handler = match ret_kind {
        ReturnKind::Result |
        ReturnKind::Option => quote! {
            result.map(|_| this)
        },
        ReturnKind::Plain => quote! { this },
        ReturnKind::Unknown => unreachable!(),
    };

    let result_storage = match ret_kind {
        ReturnKind::Result |
        ReturnKind::Option => quote! {
            let result =
        },
        ReturnKind::Plain => quote! { },
        ReturnKind::Unknown => unreachable!(),
    };

    quote! {
        #func

        /// In place initialization
        #vis fn initialize<#lifetime_name>(mut uninit: ::std::pin::Pin<&#lifetime_name  mut ::std::mem::MaybeUninit<Self>> #(, #args)*) -> #ret_type {
            let mut this = unsafe { uninit.map_unchecked_mut(|v| v.assume_init_mut()) };

            this.as_mut().__initialize_callbacks();
            #result_storage this.as_mut().#fn_name(#(#arg_idents),*);

            #result_handler
        }
    }
    .into()
}

