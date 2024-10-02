pub use camino::Utf8PathBuf;
pub use compact_str::CompactString as XString;
pub use indexmap::IndexMap;
pub use musli::{Decode, Encode};
pub use serde::{Deserialize, Serialize};

macro_rules! redb_value {
    (
      $t:ident, name: $name:literal,
      read_err: $read_err:literal, write_err: $write_err:literal
    ) => {
        impl ::redb::Value for $t {
            type SelfType<'a>
                = Self
            where
                Self: 'a;

            type AsBytes<'a>
                = Vec<u8>
            where
                Self: 'a;

            fn fixed_width() -> Option<usize> {
                None
            }

            fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
            where
                Self: 'a,
            {
                ::musli::storage::from_slice(data).expect($read_err)
            }

            fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
            where
                Self: 'a,
                Self: 'b,
            {
                ::musli::storage::to_vec(value).expect($write_err)
            }

            fn type_name() -> redb::TypeName {
                ::redb::TypeName::new($name)
            }
        }
    };
    (@key
      $t:ident, name: $name:literal,
      read_err: $read_err:literal, write_err: $write_err:literal
    ) => {
        redb_value!($t, name: $name, read_err: $read_err, write_err: $write_err);
        impl ::redb::Key for $t {
            fn compare(data1: &[u8], data2: &[u8]) -> ::std::cmp::Ordering {
                data1.cmp(data2)
            }
        }
    };
}
