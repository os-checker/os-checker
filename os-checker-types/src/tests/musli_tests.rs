use crate::Result;
use musli::{Decode, Encode};

#[derive(Debug, PartialEq, Encode, Decode)]
enum A1 {
    A,
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum A2 {
    A,
    B,
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum A3 {
    A,
    C,
    B,
}

#[test]
fn append_variant_by_default() -> Result<()> {
    let value = A1::A;
    let mut v = Vec::<u8>::new();
    musli::storage::encode(&mut v, &value)?;

    ensure!(v.len() == 1, "variant size of encoding changes");

    let decoded1: A1 = musli::storage::decode(v.as_slice())?;
    assert_eq!(value, decoded1);

    let decoded2: A2 = musli::storage::decode(v.as_slice())?;
    // NOTE: appending a variant is a non-breaking change
    assert_eq!(A2::A, decoded2);

    Ok(())
}

#[test]
fn insert_variant_by_default() -> Result<()> {
    let value = A2::B;
    let mut v = Vec::<u8>::new();
    musli::storage::encode(&mut v, &value)?;

    ensure!(v.len() == 1, "variant size of encoding changes");

    let data = v.as_slice();
    assert!(musli::storage::decode::<_, A1>(data).is_err());

    let decoded2: A2 = musli::storage::decode(data)?;
    assert_eq!(value, decoded2);

    let decoded3: A3 = musli::storage::decode(data)?;
    // NOTE: inserting a variant breaks the variant's meaning
    assert_eq!(A3::C, decoded3);

    Ok(())
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[musli(name_all = "name")]
enum B1 {
    A,
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[musli(name_all = "name")]
enum B2 {
    A,
    B,
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[musli(name_all = "name")]
enum B3 {
    A,
    C,
    B,
}

#[test]
fn append_variant_name_all() -> Result<()> {
    let value = B1::A;
    let mut v = Vec::<u8>::new();
    musli::storage::encode(&mut v, &value)?;

    ensure!(v.len() == 2, "variant size of encoding changes");

    let decoded1: B1 = musli::storage::decode(v.as_slice())?;
    assert_eq!(value, decoded1);

    let decoded2: B2 = musli::storage::decode(v.as_slice())?;
    // NOTE: appending a variant is a non-breaking change
    assert_eq!(B2::A, decoded2);

    Ok(())
}

#[test]
fn insert_variant_name_all() -> Result<()> {
    let value = B2::B;
    let mut v = Vec::<u8>::new();
    musli::storage::encode(&mut v, &value)?;

    ensure!(v.len() == 2, "variant size of encoding changes");

    let data = v.as_slice();
    assert!(musli::storage::decode::<_, A1>(data).is_err());

    let decoded2: B2 = musli::storage::decode(data)?;
    assert_eq!(value, decoded2);

    let decoded3: B3 = musli::storage::decode(data)?;
    // NOTE: inserting a variant keeps the variant's meaning,
    // thus it's also a non-breaking change
    assert_eq!(B3::B, decoded3);

    Ok(())
}
