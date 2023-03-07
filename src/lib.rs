#![allow(incomplete_features)]
#![feature(decl_macro)]
#![feature(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]

pub mod lending_stream;
pub mod lending_stream_ext;

pub use lending_stream::LendingStream;
pub use lending_stream_ext::LendingStreamExt;
