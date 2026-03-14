use proc_macro::TokenStream;

mod listener;
mod ptr;

#[proc_macro_derive(PtrWrapper)]
/// Gives conversions between the tuple type and ptrs to inner type
/// type needs to be kinda like:
/// Type(NonNull<T>)
pub fn derive_ptr_wrapper(input: TokenStream) -> TokenStream {
    ptr::derive_ptr_wrapper(input)
}

#[proc_macro_attribute]
/// C ptr conversions for custom type
pub fn c_ptr(attr: TokenStream, item: TokenStream) -> TokenStream {
    ptr::c_ptr(attr, item)
}

#[proc_macro_derive(FromPtr)]
/// Gives conversions between the tuple type and ptrs to inner type
/// Type(T)
pub fn from_ptr(input: TokenStream) -> TokenStream {
    ptr::from_ptr(input)
}

#[proc_macro_attribute]
/// Implement drop logic by calling C function
/// type needs to be kinda like:
/// Type(NonNull<T>)
pub fn c_drop(attr: TokenStream, item: TokenStream) -> TokenStream {
    ptr::c_drop(attr, item)
}

#[proc_macro_derive(WlListeners, attributes(listener))]
pub fn derive_wl_listeners(input: TokenStream) -> TokenStream {
    listener::derive_wl_listeners(input)
}

#[proc_macro_attribute]
pub fn initialization(attr: TokenStream, item: TokenStream) -> TokenStream {
    listener::initialization(attr, item)
}
