use crate::Result;
use musli::{Decode, Encode};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
enum A1 {
    A,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
enum A2 {
    A,
    B,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
enum A3 {
    A,
    C,
    B,
}

#[test]
fn append_variant() -> Result<()> {
    let value = A1::A;
    let mut v = Vec::<u8>::new();
    musli::storage::encode(&mut v, &value)?;

    dbg!(v.len());

    let decoded1: A1 = musli::storage::decode(v.as_slice())?;
    assert_eq!(value, decoded1);

    let decoded2: A2 = musli::storage::decode(v.as_slice())?;
    // NOTE: appending a variant is non-breaking change
    assert_eq!(A2::A, decoded2);

    Ok(())
}

#[test]
fn insert_variant() -> Result<()> {
    let value = A2::B;
    let mut v = Vec::<u8>::new();
    musli::storage::encode(&mut v, &value)?;

    dbg!(v.len());

    let data = v.as_slice();
    assert!(musli::storage::decode::<_, A1>(data).is_err());

    let decoded2: A2 = musli::storage::decode(data)?;
    assert_eq!(value, decoded2);

    let decoded3: A3 = musli::storage::decode(data)?;
    // NOTE: inserting a variant breaks the variant's meaning
    assert_eq!(A3::C, decoded3);

    Ok(())
}
