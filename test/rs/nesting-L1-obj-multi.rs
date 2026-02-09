Value::Object(HashMap::from([
    ("a".into(), Value::Integer(42.into())),
    ("b".into(), Value::String("hello".into())),
    ("c".into(), Value::Bytes(vec![0xb0, 0xb5])),
]))
