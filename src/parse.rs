//! Helper macros for parsing packed structs.
//!
//! The format is pretty much like `bincode` for structs,
//! but we do not want to rely on bincode just for this.
#![allow(unused_macros, dead_code)]

use anyhow::Result;
use byteordered::{byteorder::ReadBytesExt, ByteOrdered, Endian};

/// Declare a [`Parseable`] struct.
///
/// A make-do for a derive macro. Supports only simple structs
/// without generics.
macro_rules! declare_parseable_struct {
    (
        $(#[$smeta:meta])*
            $svis:vis struct $sname:ident {
                #format => $errh:expr,
                $($fvis:vis $name:ident $(as $err:expr)? => $ty:ty $(as $ty2:ty)? ),* $(,)?
            }
    ) => {

        $(#[$smeta])* #[allow(dead_code)]
            $svis struct $sname {
                $($fvis $name: declaration_type!($ty $(as $ty2)?)),*
            }

        impl Parseable for $sname {
            type Error = anyhow::Error;
            fn parse<T: ReadBytesExt, E: Endian>(r: &mut ByteOrdered<T, E>) -> Result<Self, Self::Error> {
                parse_as_bindings!(
                    r, #format => $errh,
                    $( $name $(as $err)? => $ty $(as $ty2)? ),*
                );
                Ok($sname {
                    $( $name ),*
                })
            }
        }
    };
    (
        $(#[$smeta:meta])*
            $svis:vis struct $sname:ident {
                $($fvis:vis $name:ident $(as $err:expr)? => $ty:ty $(as $ty2:ty)? ),* $(,)?
            }
    ) => {
        declare_parseable_struct! {
            $(#[$smeta])*
                $svis struct $sname {
                    #format => |e| format!("parsing field `{}.{}`", stringify!($sname), e),
                    $($fvis $name $(as $err)? => $ty $(as $ty2)?),*
                }
        }
    };
}

/// Declare multiple [`Parseable`] structs.
macro_rules! declare_parseable_structs {
    (
        $(
            $(#[$smeta:meta])*
                $svis:vis struct $sname:ident {
                    $($tt:tt)*
                }
        )*
    ) => {
        $(
            declare_parseable_struct! {
                $(#[$smeta])*
                    $svis struct $sname {
                        $($tt)*
                    }
            }
        )*
    };
}

/// Helper macro that expands to the parsed type or the
/// converted type.
macro_rules! declaration_type {
    ($ty:ty as $ty2:ty) => {
        $ty2
    };
    ($ty:ty) => {
        $ty
    };
}

/// Generate `let` bindings by parsing a reader.
macro_rules! parse_as_bindings {
    (
        $rdr: expr $(, #format => $errh:expr)?,
        $( $name:ident $(as $err:expr)? => $ty:ty $(as $ty2:ty)? ),* $(,)?
    ) => {
        #[allow(unused_parens)]
        let ($($name),*) = parse_from_reader!(
            $rdr $(, $errh)?,
            $( $ty $(as $ty2)? => stringify_binding!($name $(as $err)?) ),*
        );
    };
}

/// Helper macro to generate error context as a given
/// expression, or default to identifier name.
macro_rules! stringify_binding {
    ($name: ident as $err:expr) => {
        $err
    };
    ($name: ident) => {
        stringify!($name)
    };
}

/// Generate expression that evaluates to tuple of values
/// parsed from a reader.
macro_rules! parse_from_reader {
    ($rdr: expr, $errh: expr, $( $ty:ty $(as $ty2:ty)? => $err:expr ),* $(,)?) => {{
        let mut rdr = $rdr;
        ($(
            anyhow::Context::with_context(
                <$ty as crate::parse::Parseable>::parse(&mut rdr), || ($errh)($err)
            )? $(as $ty2)?
        ),*)
    }};
    ($rdr: expr, $( $ty:ty $(as $ty2:ty)? => $err:expr ),* $(,)?) => {{
        parse_from_reader!($rdr, |e| format!("field `{}`", e), $( $ty $(as $ty2)? => $err ),*)
    }};
}

pub(crate) trait Parseable: Sized {
    type Error;
    fn parse<T: ReadBytesExt, E: Endian>(r: &mut ByteOrdered<T, E>) -> Result<Self, Self::Error>;
}

use std::{error::Error, io::Error as IOError};
macro_rules! impl_parseable {
    ($ty:ty, $method:ident) => {
        impl Parseable for $ty {
            type Error = IOError;
            fn parse<T: ReadBytesExt, E: Endian>(
                r: &mut ByteOrdered<T, E>,
            ) -> Result<Self, IOError> {
                r.$method()
            }
        }
    };
}

impl_parseable!(u8, read_u8);
impl_parseable!(i8, read_i8);
impl_parseable!(u16, read_u16);
impl_parseable!(i16, read_i16);
impl_parseable!(u32, read_u32);
impl_parseable!(i32, read_i32);
impl_parseable!(u64, read_u64);
impl_parseable!(i64, read_i64);
impl_parseable!(f64, read_f64);
impl_parseable!(f32, read_f32);

impl<Ty, const N: usize> Parseable for [Ty; N]
where
    Ty: Parseable,
    <Ty as Parseable>::Error: Send + Sync + Error + 'static,
    [Ty; N]: Default,
{
    type Error = anyhow::Error;
    fn parse<T: ReadBytesExt, E: Endian>(r: &mut ByteOrdered<T, E>) -> Result<Self, Self::Error> {
        let mut out: [Ty; N] = Default::default();
        for i in out.iter_mut().take(N) {
            *i = Ty::parse(r)?;
        }
        Ok(out)
    }
}
