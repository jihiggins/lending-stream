#![allow(incomplete_features)]
#![feature(decl_macro)]
#![feature(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

pub mod lending_stream;
pub mod lending_stream_ext;

pub use lending_stream::LendingStream;
pub use lending_stream_ext::LendingStreamExt;
