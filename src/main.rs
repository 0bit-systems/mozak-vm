use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(
    // This will generate a PartialEq impl between our unarchived and archived
    // types:
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To use the safe
    // API, you have to derive CheckBytes for the archived type:
    check_bytes,
)]
// Derives can be passed through to the generated type:
#[archive_attr(derive(Debug))]
struct Test {
    int: u8,
    string: String,
    option: Option<Vec<i32>>,
}

fn main() {
    let value = Test {
        int: 42,
        string: "hello world".to_string(),
        option: Some(vec![1, 2, 3, 4]),
    };

    // Serializing is as easy as a single function call
    let bytes = rkyv::to_bytes::<_, 256>(&value).unwrap();

    // Or you can customize your serialization for better performance
    // and compatibility with #![no_std] environments
    use rkyv::ser::{serializers::AllocSerializer, Serializer};

    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&value).unwrap();
    let bytes = serializer.into_serializer().into_inner();

    // You can use the safe API for fast zero-copy deserialization
    let archived = rkyv::check_archived_root::<Test>(&bytes[..]).unwrap();
    assert_eq!(archived, &value);

    // Or you can use the unsafe API for maximum performance
    let archived = unsafe { rkyv::archived_root::<Test>(&bytes[..]) };
    assert_eq!(archived, &value);

    // And you can always deserialize back to the original type
    let deserialized: Test = archived.deserialize(&mut rkyv::Infallible).unwrap();
    assert_eq!(deserialized, value);
}
