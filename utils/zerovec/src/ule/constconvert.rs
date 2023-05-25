use crate::ule::AsULE;
use core::marker::PhantomData;

// TODO: rework this doc. probably mostly should be moved to ConstAsULE.
/// Workaround for the lack of const traits. Types `T` and `U` for which there exists a
/// `ConstConvert::<T, U>::aligned_to_unaligned` can be converted from `T` to `U` at compile-time.
///
/// This is necessary in part because the aligned-to-unaligned relationship is many-to-one.
/// /*
/// TODO: Talk about how this is only used for ULE types where it's difficult to define a canonical
/// AsULE<=>ULE relationship. e.g. char <=> CharULE is easy, but u32 or i32 <=> RawBytes<4>?
/// */
#[allow(dead_code, missing_debug_implementations)]
pub struct ConstConvert<T, U>
where
    T: AsULE<ULE = U>,
{
    _phantom: PhantomData<(T, U)>,
}
