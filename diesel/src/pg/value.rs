use std::cell::RefCell;
use std::num::NonZeroU32;
use std::ops::Range;

use super::PgMetadataLookup;

/// Raw postgres value as received from the database
#[derive(Clone, Copy)]
#[allow(missing_debug_implementations)]
#[cfg(feature = "postgres_backend")]
pub struct PgValue<'a> {
    raw_value: &'a [u8],
    type_oid_lookup: &'a dyn TypeOidLookup,
    metadata_lookup: &'a dyn PgMetadataLookupNonMut,
}

trait PgMetadataLookupNonMut {
    fn lookup_type(&self, type_name: &str, schema: Option<&str>) -> super::PgTypeMetadata;
}
impl<L: PgMetadataLookup> PgMetadataLookupNonMut for RefCell<&'_ mut L> {
    fn lookup_type(&self, type_name: &str, schema: Option<&str>) -> super::PgTypeMetadata {
        self.borrow_mut().lookup_type(type_name, schema)
    }
}

/// This is a helper trait to defer a type oid
/// lookup to a later point in time
///
/// This is mainly used in the `PgConnection`
/// implementation so that we do not need to call
/// into libpq if we do not need the type oid.
///
/// Backend implementations based on pure rustc
/// database connection crates can likely reuse
/// the implementation for `NonZeroU32` here instead
/// of providing their own custom implementation
#[cfg_attr(
    doc_cfg,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[allow(unreachable_pub)]
pub trait TypeOidLookup {
    /// Lookup the type oid for the current value
    fn lookup(&self) -> NonZeroU32;
}

impl<F> TypeOidLookup for F
where
    F: Fn() -> NonZeroU32,
{
    fn lookup(&self) -> NonZeroU32 {
        (self)()
    }
}

impl<'a> TypeOidLookup for PgValue<'a> {
    fn lookup(&self) -> NonZeroU32 {
        self.type_oid_lookup.lookup()
    }
}

impl TypeOidLookup for NonZeroU32 {
    fn lookup(&self) -> NonZeroU32 {
        *self
    }
}

impl<'a> PgValue<'a> {
    #[cfg(test)]
    pub(crate) fn for_test(raw_value: &'a [u8]) -> Self {
        #[allow(unsafe_code)] // that's actual safe
        static FAKE_OID: NonZeroU32 = unsafe {
            // 42 != 0, so this is actually safe
            NonZeroU32::new_unchecked(42)
        };
        Self {
            raw_value,
            type_oid_lookup: &FAKE_OID,
        }
    }

    /// Create a new instance of `PgValue` based on a byte buffer
    /// and a way to receive information about the type of the value
    /// represented by the buffer
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn new(raw_value: &'a [u8], type_oid_lookup: &'a dyn TypeOidLookup) -> Self {
        Self::new_internal(raw_value, type_oid_lookup)
    }

    pub(in crate::pg) fn new_internal(
        raw_value: &'a [u8],
        type_oid_lookup: &'a dyn TypeOidLookup,
        metadata_lookup: &'a RefCell<&mut impl PgMetadataLookup>,
    ) -> Self {
        Self {
            raw_value,
            type_oid_lookup,
            metadata_lookup,
        }
    }

    /// Get the underlying raw byte representation
    pub fn as_bytes(&self) -> &'a [u8] {
        self.raw_value
    }

    /// Get the type oid of this value
    pub fn get_oid(&self) -> NonZeroU32 {
        self.type_oid_lookup.lookup()
    }

    pub(crate) fn subslice(&self, range: Range<usize>) -> Self {
        Self {
            raw_value: &self.raw_value[range],
            ..*self
        }
    }
}

impl PgMetadataLookup for PgValue<'_> {
    fn lookup_type(&mut self, type_name: &str, schema: Option<&str>) -> super::PgTypeMetadata {
        self.metadata_lookup.lookup_type(type_name, schema)
    }

    fn as_any<'a>(&mut self) -> &mut (dyn std::any::Any + 'a)
    where
        Self: 'a,
    {
        self
    }
}
