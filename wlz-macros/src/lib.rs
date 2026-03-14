use proc_macro::TokenStream;

mod listener;
mod ptr;

#[proc_macro_derive(PtrWrapper)]
pub fn derive_ptr_wrapper(input: TokenStream) -> TokenStream {
    ptr::derive_ptr_wrapper(input)
}

#[proc_macro_derive(WlListeners, attributes(listener))]
pub fn derive_wl_listeners(input: TokenStream) -> TokenStream {
    listener::derive_wl_listeners(input)
}

#[proc_macro_attribute]
pub fn c_drop(attr: TokenStream, item: TokenStream) -> TokenStream {
    ptr::c_drop(attr, item)
}

#[proc_macro_attribute]
pub fn c_ptr(attr: TokenStream, item: TokenStream) -> TokenStream {
    ptr::c_ptr(attr, item)
}

#[proc_macro_derive(FromPtr)]
pub fn from_ptr(input: TokenStream) -> TokenStream {
    ptr::from_ptr(input)
}

#[proc_macro_attribute]
pub fn initialization(attr: TokenStream, item: TokenStream) -> TokenStream {
    listener::initialization(attr, item)
}
