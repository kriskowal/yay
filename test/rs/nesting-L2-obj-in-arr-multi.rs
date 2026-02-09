Value::Array(vec![
    Value::Object(HashMap::from([
        ("a".into(), Value::Integer(42.into())),
        ("b".into(), Value::String("hello".into())),
    ])),
    Value::Object(HashMap::from([("c".into(), Value::Integer(42.into()))])),
])
